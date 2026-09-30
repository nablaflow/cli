use color_eyre::eyre;
use reqwest::StatusCode;
use std::{
    hash::{BuildHasher, RandomState},
    time::Duration,
};
use tokio::time::{self, Instant};

const BACKOFF_BASE: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Outcome of a failed attempt, deciding whether to try again.
#[derive(Debug)]
pub enum Failure {
    /// Transient failure (network, timeout, 5xx...), counts towards `max_attempts`.
    Transient(eyre::Report),
    /// A request with the same idempotency key is still being processed on the server.
    /// Does not count towards `max_attempts`, bounded by `in_progress_budget` instead.
    InProgress(eyre::Report),
    /// Retrying would not help.
    Fatal(eyre::Report),
}

impl<E: Into<eyre::Report>> From<E> for Failure {
    fn from(err: E) -> Self {
        Self::Fatal(err.into())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    pub max_attempts: u32,
    pub in_progress_budget: Duration,
}

pub async fn retry<T, F, Fut>(
    what: &str,
    policy: Policy,
    mut attempt: F,
) -> eyre::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, Failure>>,
{
    let started_at = Instant::now();
    let mut transient_failures = 0;
    let mut step = 0;

    loop {
        let err = match attempt().await {
            Ok(value) => return Ok(value),
            Err(Failure::Fatal(err)) => return Err(err),
            Err(Failure::Transient(err)) => {
                transient_failures += 1;

                if transient_failures >= policy.max_attempts {
                    return Err(err.wrap_err(format!(
                        "{what}: giving up after {transient_failures} attempts"
                    )));
                }

                err
            }
            Err(Failure::InProgress(err)) => {
                if started_at.elapsed() >= policy.in_progress_budget {
                    return Err(err.wrap_err(format!(
                        "{what}: still in progress after {:?}",
                        policy.in_progress_budget
                    )));
                }

                err
            }
        };

        let delay = backoff(step);
        step += 1;

        tracing::warn!("{what}: attempt failed, retrying in {delay:?}: {err:#}");

        time::sleep(delay).await;
    }
}

/// Statuses worth retrying the same request for.
pub fn is_transient_status(status: StatusCode) -> bool {
    status.is_server_error()
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
}

/// Exponential backoff with equal jitter: half of the delay is fixed, half is random.
fn backoff(step: u32) -> Duration {
    let delay = BACKOFF_BASE
        .saturating_mul(2u32.saturating_pow(step))
        .min(BACKOFF_MAX);

    let half = delay / 2;
    let random = RandomState::new().hash_one(step);

    #[allow(clippy::cast_precision_loss)]
    let jitter = half.mul_f64(random as f64 / u64::MAX as f64);

    half + jitter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded() {
        for step in 0..100 {
            let delay = backoff(step);
            let ceiling = BACKOFF_BASE
                .saturating_mul(2u32.saturating_pow(step))
                .min(BACKOFF_MAX);

            assert!(delay >= ceiling / 2, "step {step}: {delay:?}");
            assert!(delay <= ceiling, "step {step}: {delay:?}");
        }
    }
}
