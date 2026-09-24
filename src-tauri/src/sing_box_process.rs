use crate::{
    china_rules::{resource_root, ChinaRuleSets},
    credentials::ProxyCredential,
    error::AppError,
    models::{PersistedConfiguration, RuntimeMode},
    sing_box_config::{render_with_rules, SingBoxPorts},
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub const WINDOWS_AMD64_EXE_SHA256: &str =
    "b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909";
#[cfg(windows)]
pub const WINDOWS_AMD64_DLL_SHA256: &str =
    "3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928";

pub struct SingBoxProcess {
    child: Child,
    #[cfg(windows)]
    _job: crate::sing_box_windows_job::CoreJob,
    _config: tempfile::NamedTempFile,
    control_secret: String,
    pub proxy_port: u16,
    pub control_port: u16,
    _private_dir: PrivateRuntimeDir,
}

struct PrivateRuntimeDir(PathBuf);

impl Drop for PrivateRuntimeDir {
    fn drop(&mut self) {
        #[cfg(windows)]
        let _ = std::fs::remove_file(self.0.join("job-owned"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

impl SingBoxProcess {
    pub fn start(
        binary: &Path,
        expected_sha256: &str,
        runtime_root: &Path,
        configuration: &PersistedConfiguration,
        mode: RuntimeMode,
        credentials: &HashMap<String, ProxyCredential>,
    ) -> Result<Self, AppError> {
        verify_binary(binary, expected_sha256)?;
        #[cfg(windows)]
        if expected_sha256 == WINDOWS_AMD64_EXE_SHA256 {
            let library = binary.with_file_name("libcronet.dll");
            verify_binary(&library, WINDOWS_AMD64_DLL_SHA256)?;
        }
        std::fs::create_dir_all(runtime_root).map_err(|_| process_error("无法建立内核运行目录"))?;
        let private_path = runtime_root.join(format!("run-{}", uuid::Uuid::new_v4()));
        ensure_private_dir(&private_path)?;
        let private_dir = PrivateRuntimeDir(private_path);
        let proxy_port = available_port()?;
        let control_port = available_port_except(proxy_port)?;
        let mut random = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut random);
        let control_secret = hex::encode(random);
        let china_rules = if mode == RuntimeMode::Rules && configuration.china_direct_enabled {
            Some(ChinaRuleSets::verify(&resource_root(binary)?)?)
        } else {
            None
        };
        let config_json = render_with_rules(
            configuration,
            mode,
            SingBoxPorts {
                proxy: proxy_port,
                control: control_port,
            },
            &control_secret,
            credentials,
            china_rules.as_ref(),
        )?;
        let mut config = tempfile::Builder::new()
            .prefix("sing-box-")
            .suffix(".json")
            .tempfile_in(&private_dir.0)
            .map_err(|_| process_error("无法创建内核临时配置"))?;
        config
            .write_all(config_json.as_bytes())
            .and_then(|()| config.flush())
            .map_err(|_| process_error("无法写入内核临时配置"))?;
        let mut command = Command::new(binary);
        command
            .arg("run")
            .arg("-c")
            .arg(config.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        #[cfg(windows)]
        let job = crate::sing_box_windows_job::CoreJob::new()?;
        let child = command
            .spawn()
            .map_err(|_| process_error("无法启动已验证的代理内核"))?;
        #[cfg(windows)]
        let child = {
            let mut child = child;
            if let Err(error) = job.attach(&child) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            child
        };
        #[cfg(windows)]
        if std::fs::write(private_dir.0.join("job-owned"), b"").is_err() {
            let mut child = child;
            let _ = child.kill();
            let _ = child.wait();
            return Err(process_error("无法记录受保护的内核运行目录"));
        }
        let mut process = Self {
            child,
            #[cfg(windows)]
            _job: job,
            _config: config,
            control_secret,
            proxy_port,
            control_port,
            _private_dir: private_dir,
        };
        if let Err(error) = process.await_ready(Duration::from_secs(8)) {
            process.stop();
            return Err(error);
        }
        Ok(process)
    }

    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    #[cfg(test)]
    pub fn config_path(&self) -> PathBuf {
        self._config.path().to_owned()
    }

    pub fn is_running(&mut self) -> Result<bool, AppError> {
        self.child
            .try_wait()
            .map(|result| result.is_none())
            .map_err(|_| process_error("无法检查受管内核进程"))
    }

    pub fn connections_json(&mut self) -> Result<serde_json::Value, AppError> {
        if !self.is_running()? {
            return Err(process_error("受管内核已退出"));
        }
        let response = self.request_connections(true)?;
        serde_json::from_slice(&response).map_err(|_| process_error("内核活跃连接响应无法解析"))
    }

    fn await_ready(&mut self, timeout: Duration) -> Result<(), AppError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if !self.is_running()? {
                return Err(process_error("代理内核在监听就绪前退出"));
            }
            if TcpStream::connect_timeout(
                &([127, 0, 0, 1], self.proxy_port).into(),
                Duration::from_millis(100),
            )
            .is_ok()
                && self.request_connections(true).is_ok()
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(process_error("代理内核监听和控制接口健康检查超时"))
    }

    fn request_connections(&self, authorized: bool) -> Result<Vec<u8>, AppError> {
        let mut stream = TcpStream::connect_timeout(
            &([127, 0, 0, 1], self.control_port).into(),
            Duration::from_millis(200),
        )
        .map_err(|_| process_error("内核控制接口不可访问"))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|_| process_error("无法设置控制接口超时"))?;
        let auth = if authorized {
            format!("Authorization: Bearer {}\r\n", self.control_secret)
        } else {
            String::new()
        };
        let request = format!(
            "GET /connections HTTP/1.1\r\nHost: 127.0.0.1\r\n{auth}Connection: close\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| process_error("无法读取内核控制接口"))?;
        let mut response = Vec::new();
        stream
            .take(1024 * 1024)
            .read_to_end(&mut response)
            .map_err(|_| process_error("无法读取内核控制接口"))?;
        let separator = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| process_error("内核控制接口响应格式错误"))?;
        if !response.starts_with(b"HTTP/1.1 200 ") && !response.starts_with(b"HTTP/1.0 200 ") {
            return Err(process_error("内核控制接口未通过授权或健康检查"));
        }
        Ok(response[separator + 4..].to_vec())
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Only called after obtaining the exclusive application session lease.
/// A job-owned core cannot outlive that lease's previous process, so these
/// private directories are no longer in use after a crash.
#[cfg(windows)]
pub fn cleanup_stale_runtime_dirs(root: &Path) -> Result<(), AppError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(process_error("无法检查遗留内核配置")),
    };
    for entry in entries {
        let entry = entry.map_err(|_| process_error("无法检查遗留内核配置"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(id) = name.strip_prefix("run-") else {
            continue;
        };
        if uuid::Uuid::parse_str(id).is_err() {
            continue;
        }
        let path = entry.path();
        if !entry
            .file_type()
            .map_err(|_| process_error("无法检查遗留内核配置"))?
            .is_dir()
        {
            continue;
        }
        // Older versions did not own a kill-on-close job: their orphaned
        // processes could still be using the config. Never delete those runs.
        if !path.join("job-owned").is_file() {
            continue;
        }
        std::fs::remove_dir_all(path).map_err(|_| process_error("无法清理遗留内核配置"))?;
    }
    Ok(())
}

impl Drop for SingBoxProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn available_port() -> Result<u16, AppError> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|_| process_error("无法分配回环监听端口"))
}

fn available_port_except(other: u16) -> Result<u16, AppError> {
    for _ in 0..4 {
        let port = available_port()?;
        if port != other {
            return Ok(port);
        }
    }
    Err(process_error("无法分配独立的控制接口端口"))
}

pub fn verify_binary(binary: &Path, expected_sha256: &str) -> Result<(), AppError> {
    let mut file = File::open(binary).map_err(|_| process_error("代理内核二进制缺失"))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|_| process_error("无法校验代理内核二进制"))?;
    let actual = hex::encode(hasher.finalize());
    if actual != expected_sha256 {
        return Err(process_error("代理内核版本或校验值不匹配"));
    }
    Ok(())
}

fn ensure_private_dir(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(|_| process_error("无法建立内核私有目录"))?;
        let metadata = path
            .symlink_metadata()
            .map_err(|_| process_error("无法检查内核私有目录"))?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(process_error("内核配置目录权限不安全"));
        }
    }
    #[cfg(windows)]
    {
        crate::sing_box_windows_acl::create_private_runtime_dir(path)?;
    }
    Ok(())
}

fn process_error(message: &str) -> AppError {
    AppError {
        code: "runtime_error".into(),
        message: message.into(),
        fields: Vec::new(),
    }
}

#[cfg(test)]
#[path = "sing_box_process_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sing_box_integration_tests.rs"]
mod integration_tests;
