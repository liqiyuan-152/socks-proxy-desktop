use crate::error::AppError;
use std::{
    io::{Read, Seek, SeekFrom},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 64 * 1024;

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Reap on every exit path, including timeout and output/read failures.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn run(command: &mut Command, timeout: Duration) -> Result<String, AppError> {
    let failed = || AppError::unavailable("规则测试进程执行失败");
    let mut stderr = tempfile::tempfile().map_err(|_| failed())?;
    let writer = stderr.try_clone().map_err(|_| failed())?;
    let mut child = OwnedChild(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(writer))
            .spawn()
            .map_err(|_| failed())?,
    );
    let deadline = Instant::now() + timeout;
    loop {
        if stderr.metadata().map_err(|_| failed())?.len() > OUTPUT_LIMIT {
            return Err(AppError::unavailable("规则测试输出超过限制"));
        }
        if let Some(status) = child.0.try_wait().map_err(|_| failed())? {
            if !status.success() {
                return Err(failed());
            }
            break;
        }
        if Instant::now() >= deadline {
            return Err(AppError::unavailable("规则测试超时，请稍后重试"));
        }
        thread::sleep(Duration::from_millis(10));
    }
    stderr.seek(SeekFrom::Start(0)).map_err(|_| failed())?;
    let mut output = String::new();
    stderr
        .take(OUTPUT_LIMIT + 1)
        .read_to_string(&mut output)
        .map_err(|_| failed())?;
    if output.len() as u64 > OUTPUT_LIMIT {
        return Err(AppError::unavailable("规则测试输出超过限制"));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "rule_match_process_tests.rs"]
mod tests;
