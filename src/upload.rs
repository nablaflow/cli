//! Uploads files to presigned urls (e.g. S3 PUTs).
//!
//! Uploads go through two lanes: small files upload in parallel, while large files tend to
//! saturate bandwidth and upload with low concurrency (sequentially by default).

use crate::{
    progress::{ProgressFn, ProgressScope},
    retry::{self, Failure, Policy, retry},
};
use bytesize::ByteSize;
use color_eyre::eyre::{self, WrapErr};
use futures_util::StreamExt;
use reqwest::{StatusCode, header::CONTENT_LENGTH};
use std::{num::NonZeroUsize, path::Path, sync::Arc, time::Duration};
use tokio::{fs::File, sync::Semaphore};
use tokio_util::io::ReaderStream;

const CHUNK_SIZE: usize = 1024 * 1024;
const NOTIFY_PROGRESS_EVERY_BYTES: u64 = 2 * 1024 * 1024;

const RETRY_POLICY: Policy = Policy {
    max_attempts: 5,
    in_progress_budget: Duration::ZERO,
};

#[derive(Debug, Clone)]
pub struct Uploader {
    client: reqwest::Client,
    small_files_lane: Arc<Semaphore>,
    large_files_lane: Arc<Semaphore>,
    large_file_threshold: ByteSize,
}

impl Uploader {
    pub fn new(
        client: reqwest::Client,
        small_files_concurrency: NonZeroUsize,
        large_files_concurrency: NonZeroUsize,
        large_file_threshold: ByteSize,
    ) -> Self {
        Self {
            client,
            small_files_lane: Arc::new(Semaphore::new(
                small_files_concurrency.get(),
            )),
            large_files_lane: Arc::new(Semaphore::new(
                large_files_concurrency.get(),
            )),
            large_file_threshold,
        }
    }

    /// Waits for a free slot in the lane matching `size`, then PUTs the file, retrying on
    /// transient failures.
    ///
    /// `fetch_url` is called before every attempt, since presigned urls expire.
    /// Progress of failed or aborted attempts gets rolled back.
    pub async fn upload<F, Fut>(
        &self,
        path: &Path,
        size: ByteSize,
        mut fetch_url: F,
        progress: &ProgressFn,
    ) -> eyre::Result<()>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<String, Failure>>,
    {
        let lane = if size >= self.large_file_threshold {
            &self.large_files_lane
        } else {
            &self.small_files_lane
        };

        // NOTE: held across retries, so that large files keep uploading one at a time.
        let _permit = lane.acquire().await?;

        retry(
            &format!("uploading `{}`", path.display()),
            RETRY_POLICY,
            || {
                let url = fetch_url();

                async {
                    // NOTE: rolls back progress when the attempt fails or gets aborted.
                    let scope = ProgressScope::new(progress.clone());

                    self.put(&url.await?, path, &scope.progress_fn()).await?;

                    scope.commit();

                    Ok(())
                }
            },
        )
        .await?;

        tracing::info!("uploaded `{}`", path.display());

        Ok(())
    }

    async fn put(
        &self,
        url: &str,
        path: &Path,
        progress: &ProgressFn,
    ) -> Result<(), Failure> {
        let fd = File::open(path)
            .await
            .wrap_err_with(|| format!("opening `{}`", path.display()))?;
        let len = fd.metadata().await?.len();

        let mut reader_stream = ReaderStream::with_capacity(fd, CHUNK_SIZE);
        let progress = progress.clone();

        let body_stream = async_stream::stream! {
            let mut pending = 0;

            while let Some(chunk) = reader_stream.next().await {
                if let Ok(chunk) = &chunk {
                    pending += chunk.len() as u64;

                    if pending >= NOTIFY_PROGRESS_EVERY_BYTES {
                        report_progress(&progress, pending);
                        pending = 0;
                    }
                }

                yield chunk;
            }

            report_progress(&progress, pending);
        };

        let res = self
            .client
            .put(url)
            .body(reqwest::Body::wrap_stream(body_stream))
            .header(CONTENT_LENGTH, len.to_string())
            .send()
            .await
            .map_err(|err| {
                if err.is_builder() {
                    Failure::Fatal(err.into())
                } else {
                    Failure::Transient(err.into())
                }
            })?;

        let status = res.status();

        if status.is_success() {
            return Ok(());
        }

        let body = res.text().await.unwrap_or_default();
        let err = eyre::eyre!("file server responded with {status}: {body}");

        // S3 answers 400 with `RequestTimeout` when the connection stalls.
        if retry::is_transient_status(status)
            || (status == StatusCode::BAD_REQUEST
                && body.contains("<Code>RequestTimeout</Code>"))
        {
            Err(Failure::Transient(err))
        } else {
            Err(Failure::Fatal(err))
        }
    }
}

fn report_progress(progress: &ProgressFn, bytes: u64) {
    if bytes == 0 {
        return;
    }

    progress(i64::try_from(bytes).unwrap_or(i64::MAX));
}
