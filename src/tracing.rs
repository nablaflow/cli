use crate::args::Args;
use color_eyre::eyre::{self, WrapErr};
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, PoisonError},
};
use tokio::sync::Notify;
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;

/// How many log lines are kept in memory for the interactive UI. The log file
/// on disk keeps everything.
const LOG_BUFFER_MAX_LINES: usize = 10_000;

static LOG_BUFFER: OnceLock<Arc<LogBuffer>> = OnceLock::new();

pub fn init(args: &Args) -> eyre::Result<()> {
    let max_level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    if args.is_interactive_ui() {
        // NOTE: writing to stderr would corrupt the UI, so logs go to a file (a temporary one
        // if not specified) and to an in-memory buffer that the UI can display.
        let path = args.log_to_path.clone().unwrap_or_else(temp_log_path);
        let buffer = Arc::new(LogBuffer::new(path.clone()));

        tracing_subscriber::fmt()
            .with_writer(open_log_file(&path)?.and(Arc::clone(&buffer)))
            .with_ansi(false)
            .with_max_level(max_level)
            .init();

        let _ = LOG_BUFFER.set(buffer);
    } else if let Some(ref path) = args.log_to_path {
        tracing_subscriber::fmt()
            .with_writer(open_log_file(path)?)
            .with_ansi(false)
            .with_max_level(max_level)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(io::stderr)
            .with_max_level(max_level)
            .init();
    }

    Ok(())
}

/// In-memory log buffer, only available when running an interactive UI.
pub fn log_buffer() -> Option<Arc<LogBuffer>> {
    LOG_BUFFER.get().cloned()
}

fn open_log_file(path: &Path) -> eyre::Result<std::fs::File> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .wrap_err_with(|| format!("opening log file `{}`", path.display()))
}

fn temp_log_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nf-{}-{}.log",
        chrono::Local::now().format("%Y%m%dT%H%M%S"),
        std::process::id()
    ))
}

#[derive(Debug)]
pub struct LogBuffer {
    path: PathBuf,
    inner: Mutex<LogBufferInner>,
    notify: Notify,
}

#[derive(Debug, Default)]
struct LogBufferInner {
    lines: VecDeque<String>,
    partial: String,
    /// Total lines ever appended, used to detect changes and evictions.
    appended: usize,
}

impl LogBuffer {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            inner: Mutex::default(),
            notify: Notify::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Calls `f` with the buffered lines and the total number of lines ever appended.
    pub fn with_lines<R>(
        &self,
        f: impl FnOnce(&VecDeque<String>, usize) -> R,
    ) -> R {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        f(&inner.lines, inner.appended)
    }

    /// Resolves once new lines have been appended since the last call.
    pub async fn changed(&self) {
        self.notify.notified().await;
    }
}

impl Write for &LogBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let LogBufferInner {
            lines,
            partial,
            appended,
        } = &mut *inner;

        partial.push_str(&String::from_utf8_lossy(buf));

        let mut any = false;
        while let Some(idx) = partial.find('\n') {
            let line: String = partial.drain(..=idx).collect();
            lines.push_back(line.trim_end().to_owned());
            *appended += 1;
            any = true;
        }

        while lines.len() > LOG_BUFFER_MAX_LINES {
            lines.pop_front();
        }

        drop(inner);

        if any {
            // NOTE: `notify_one` stores a permit if nobody is waiting, so no line is missed.
            self.notify.notify_one();
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(buffer: &LogBuffer) -> (Vec<String>, usize) {
        buffer.with_lines(|lines, appended| {
            (lines.iter().cloned().collect(), appended)
        })
    }

    #[test]
    fn log_buffer_joins_partial_writes() {
        let buffer = LogBuffer::new(PathBuf::new());

        (&buffer).write_all(b"first\nsec").unwrap();
        (&buffer).write_all(b"ond\n").unwrap();

        assert_eq!(lines(&buffer), (vec!["first".into(), "second".into()], 2));
    }

    #[test]
    fn log_buffer_evicts_oldest_lines() {
        let buffer = LogBuffer::new(PathBuf::new());

        for i in 0..LOG_BUFFER_MAX_LINES + 5 {
            writeln!(&buffer, "{i}").unwrap();
        }

        let (lines, appended) = lines(&buffer);
        assert_eq!(appended, LOG_BUFFER_MAX_LINES + 5);
        assert_eq!(lines.len(), LOG_BUFFER_MAX_LINES);
        assert_eq!(lines[0], "5");
    }
}
