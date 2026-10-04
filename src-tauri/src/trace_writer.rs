//! 有界日志文件写入器；只管理本应用创建的固定日志文件。
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
use tracing_subscriber::fmt::MakeWriter;

const MAX_FILE_BYTES: u64 = 1024 * 1024;
const BACKUP_COUNT: usize = 4;

pub(crate) struct TraceWriter(Mutex<RotatingLog>);
struct RotatingLog {
    directory: PathBuf,
    file: Option<File>,
    bytes: u64,
    limit: u64,
}
pub(crate) struct LockedWriter<'a>(MutexGuard<'a, RotatingLog>);

impl TraceWriter {
    pub(crate) fn open(directory: &Path) -> io::Result<Self> {
        Self::with_limit(directory, MAX_FILE_BYTES)
    }
    fn with_limit(directory: &Path, limit: u64) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let path = directory.join("runtime.jsonl");
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let bytes = file.metadata()?.len();
        Ok(Self(Mutex::new(RotatingLog {
            directory: directory.into(),
            file: Some(file),
            bytes,
            limit,
        })))
    }
}
impl Write for TraceWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.make_writer().write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.make_writer().flush()
    }
}

/// 有界异步队列满时丢弃追踪记录，业务线程不等待磁盘。
pub(crate) fn asynchronous(
    directory: &Path,
) -> io::Result<(
    tracing_appender::non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    Ok(
        tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(1024)
            .lossy(true)
            .thread_name("runtime-trace-writer")
            .finish(TraceWriter::open(directory)?),
    )
}

impl<'a> MakeWriter<'a> for TraceWriter {
    type Writer = LockedWriter<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        LockedWriter(self.0.lock().unwrap_or_else(|poison| poison.into_inner()))
    }
}
impl Write for LockedWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl RotatingLog {
    fn rotate(&mut self) -> io::Result<()> {
        self.flush()?;
        self.file.take(); // Windows 必须先关闭句柄再重命名。
        let operation = (|| {
            let oldest = self.directory.join(format!("runtime.{BACKUP_COUNT}.jsonl"));
            if oldest.exists() {
                fs::remove_file(oldest)?;
            }
            for index in (1..BACKUP_COUNT).rev() {
                let from = self.directory.join(format!("runtime.{index}.jsonl"));
                if from.exists() {
                    fs::rename(
                        from,
                        self.directory.join(format!("runtime.{}.jsonl", index + 1)),
                    )?;
                }
            }
            fs::rename(
                self.directory.join("runtime.jsonl"),
                self.directory.join("runtime.1.jsonl"),
            )
        })();
        // 即使轮转失败也重新打开当前文件，后续事件仍可重试。
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join("runtime.jsonl"))?;
        self.bytes = file.metadata()?.len();
        self.file = Some(file);
        operation
    }
}
impl Write for RotatingLog {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.len() as u64 > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "trace record exceeds size limit",
            ));
        }
        if self.bytes > 0 && self.bytes.saturating_add(buffer.len() as u64) > self.limit {
            self.rotate()?;
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| io::Error::other("log unavailable"))?;
        let written = file.write(buffer)?;
        self.bytes += written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("log unavailable"))?
            .flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn asynchronous_writer_flushes_complete_records_on_guard_drop() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        let (mut writer, guard) = asynchronous(directory.path())?;
        for index in 0..100 {
            writer.write_all(format!("{{\"n\":{index}}}\n").as_bytes())?;
        }
        assert_eq!(writer.error_counter().dropped_lines(), 0);
        drop(guard);
        let content = fs::read_to_string(directory.path().join("runtime.jsonl"))?;
        assert_eq!(content.lines().count(), 100);
        for line in content.lines() {
            assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
        }
        Ok(())
    }
    #[test]
    fn saturated_queue_counts_drops_without_blocking_business_thread() -> io::Result<()> {
        struct HeldWriter {
            entered: std::sync::mpsc::Sender<()>,
            release: std::sync::mpsc::Receiver<()>,
            first: bool,
        }
        impl Write for HeldWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.first {
                    self.first = false;
                    self.entered.send(()).map_err(io::Error::other)?;
                    self.release.recv().map_err(io::Error::other)?;
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (mut writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(1)
            .lossy(true)
            .finish(HeldWriter {
                entered: entered_tx,
                release: release_rx,
                first: true,
            });
        writer.write_all(b"first\n")?;
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .map_err(io::Error::other)?;
        writer.write_all(b"queued\n")?;
        writer.write_all(b"dropped\n")?;
        assert_eq!(writer.error_counter().dropped_lines(), 1);
        release_tx.send(()).map_err(io::Error::other)?;
        drop(guard);
        Ok(())
    }
    #[test]
    fn rotates_at_record_boundary_and_bounds_backup_count() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        let writer = TraceWriter::with_limit(directory.path(), 10)?;
        for index in 0..12 {
            writer
                .make_writer()
                .write_all(format!("{{\"n\":{index}}}\n").as_bytes())?;
        }
        let entries = fs::read_dir(directory.path())?.collect::<Result<Vec<_>, _>>()?;
        assert_eq!(entries.len(), BACKUP_COUNT + 1);
        for entry in entries {
            for line in fs::read_to_string(entry.path())?.lines() {
                assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
            }
        }
        assert_eq!(
            fs::read_to_string(directory.path().join("runtime.jsonl"))?,
            "{\"n\":11}\n"
        );
        Ok(())
    }
    #[test]
    fn append_on_reopen_preserves_previous_records_and_unrelated_files() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        fs::write(directory.path().join("unrelated"), "keep")?;
        TraceWriter::open(directory.path())?
            .make_writer()
            .write_all(b"first\n")?;
        TraceWriter::open(directory.path())?
            .make_writer()
            .write_all(b"second\n")?;
        assert_eq!(
            fs::read_to_string(directory.path().join("runtime.jsonl"))?,
            "first\nsecond\n"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("unrelated"))?,
            "keep"
        );
        Ok(())
    }
    #[test]
    fn oversized_records_and_failed_rotation_preserve_existing_file() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        let writer = TraceWriter::with_limit(directory.path(), 10)?;
        writer.make_writer().write_all(b"first\n")?;
        assert!(writer.make_writer().write_all(b"01234567890").is_err());
        fs::create_dir(directory.path().join("runtime.4.jsonl"))?;
        assert!(writer.make_writer().write_all(b"second\n").is_err());
        assert_eq!(
            fs::read_to_string(directory.path().join("runtime.jsonl"))?,
            "first\n"
        );
        fs::remove_dir(directory.path().join("runtime.4.jsonl"))?;
        writer.make_writer().write_all(b"second\n")?;
        assert_eq!(
            fs::read_to_string(directory.path().join("runtime.1.jsonl"))?,
            "first\n"
        );
        Ok(())
    }
}
