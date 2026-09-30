use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI64, Ordering},
};
use tokio::sync::mpsc::{self, error::TrySendError};

/// Called with the amount of progress (e.g. bytes) made since last call, negative when
/// progress has to be discarded (e.g. a failed upload attempt).
pub type ProgressFn = Arc<dyn Fn(i64) + Send + Sync>;

pub fn no_progress() -> ProgressFn {
    Arc::new(|_| {})
}

/// Reports progress to a channel without ever blocking the caller: when the channel is full,
/// progress accumulates and a background task delivers it as soon as there is room.
pub fn channel_reporter<T: Send + 'static>(
    tx: mpsc::Sender<T>,
    to_message: fn(i64) -> T,
) -> ProgressFn {
    let reporter = Arc::new(ChannelReporter {
        tx,
        to_message,
        pending: AtomicI64::new(0),
        flushing: AtomicBool::new(false),
    });

    Arc::new(move |delta| reporter.report(delta))
}

struct ChannelReporter<T> {
    tx: mpsc::Sender<T>,
    to_message: fn(i64) -> T,
    pending: AtomicI64,
    flushing: AtomicBool,
}

impl<T: Send + 'static> ChannelReporter<T> {
    fn report(self: &Arc<Self>, delta: i64) {
        self.pending.fetch_add(delta, Ordering::Relaxed);

        if self.flushing.load(Ordering::Relaxed) {
            // Picked up by the flushing task.
            return;
        }

        let total = self.pending.swap(0, Ordering::Relaxed);

        if total == 0 {
            return;
        }

        match self.tx.try_send((self.to_message)(total)) {
            Ok(()) | Err(TrySendError::Closed(..)) => {}
            Err(TrySendError::Full(..)) => {
                self.pending.fetch_add(total, Ordering::Relaxed);
                self.flushing.store(true, Ordering::Relaxed);

                let this = self.clone();

                tokio::spawn(async move {
                    let permit = this.tx.reserve().await;

                    this.flushing.store(false, Ordering::Relaxed);

                    let total = this.pending.swap(0, Ordering::Relaxed);

                    if let Ok(permit) = permit
                        && total != 0
                    {
                        permit.send((this.to_message)(total));
                    }
                });
            }
        }
    }
}

/// Keeps track of the progress reported through it, rolling it back when dropped, unless
/// committed. Once dropped or committed, further reports are ignored: they might come from
/// work that is still being aborted.
pub struct ProgressScope {
    parent: ProgressFn,
    net: Arc<AtomicI64>,
    committed: bool,
}

impl ProgressScope {
    const CLOSED: i64 = i64::MIN;

    pub fn new(parent: ProgressFn) -> Self {
        Self {
            parent,
            net: Arc::new(AtomicI64::new(0)),
            committed: false,
        }
    }

    pub fn progress_fn(&self) -> ProgressFn {
        let parent = self.parent.clone();
        let net = self.net.clone();

        Arc::new(move |delta| {
            let still_open = net
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |net| {
                    if net == Self::CLOSED {
                        None
                    } else {
                        Some(net + delta)
                    }
                })
                .is_ok();

            if still_open {
                parent(delta);
            }
        })
    }

    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ProgressScope {
    fn drop(&mut self) {
        let net = self.net.swap(Self::CLOSED, Ordering::Relaxed);

        if !self.committed && net != Self::CLOSED && net != 0 {
            (self.parent)(-net);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter() -> (Arc<AtomicI64>, ProgressFn) {
        let total = Arc::new(AtomicI64::new(0));
        let progress: ProgressFn = {
            let total = total.clone();
            Arc::new(move |delta| {
                total.fetch_add(delta, Ordering::Relaxed);
            })
        };

        (total, progress)
    }

    #[test]
    fn scope_keeps_progress_when_committed() {
        let (total, progress) = counter();

        let scope = ProgressScope::new(progress);
        let report = scope.progress_fn();
        report(10);
        report(5);
        scope.commit();

        // Late reports are ignored.
        report(7);

        assert_eq!(total.load(Ordering::Relaxed), 15);
    }

    #[test]
    fn scope_rolls_back_progress_when_dropped() {
        let (total, progress) = counter();

        let scope = ProgressScope::new(progress);
        let report = scope.progress_fn();
        report(10);
        report(5);
        drop(scope);

        // Late reports are ignored.
        report(7);

        assert_eq!(total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn nested_scopes_roll_back_once() {
        let (total, progress) = counter();

        let outer = ProgressScope::new(progress);

        let done = ProgressScope::new(outer.progress_fn());
        done.progress_fn()(100);
        done.commit();

        let aborted = ProgressScope::new(outer.progress_fn());
        aborted.progress_fn()(40);

        assert_eq!(total.load(Ordering::Relaxed), 140);

        // Outer scope fails before the inner one gets dropped.
        drop(outer);
        assert_eq!(total.load(Ordering::Relaxed), 0);

        drop(aborted);
        assert_eq!(total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn nested_scope_rolls_back_failed_attempt_only() {
        let (total, progress) = counter();

        let outer = ProgressScope::new(progress);

        let failed_attempt = ProgressScope::new(outer.progress_fn());
        failed_attempt.progress_fn()(30);
        drop(failed_attempt);

        let attempt = ProgressScope::new(outer.progress_fn());
        attempt.progress_fn()(100);
        attempt.commit();

        outer.commit();

        assert_eq!(total.load(Ordering::Relaxed), 100);
    }

    #[tokio::test]
    async fn channel_reporter_delivers_everything_when_full() {
        let (tx, mut rx) = mpsc::channel(1);
        let progress = channel_reporter(tx, |delta| delta);

        for _ in 0..100 {
            progress(3);
        }
        progress(-50);

        let mut total = 0;
        while total != 250 {
            total += rx.recv().await.unwrap();
        }

        assert_eq!(total, 250);
        assert!(rx.try_recv().is_err());
    }
}
