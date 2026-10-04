use super::*;
use crate::sing_box_process::SingBoxProcess;
use sha2::Digest;
use std::{
    net::TcpListener,
    sync::Mutex,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Threading::{OpenProcess, WaitForSingleObject},
};

#[derive(Default)]
pub(super) struct Cores(Mutex<Vec<SingBoxProcess>>);
impl Cores {
    pub(super) fn start(
        &self,
        root: &std::path::Path,
        configuration: &PersistedConfiguration,
        mode: RuntimeMode,
        secret: crate::credentials::ProxyCredential,
    ) -> Result<u32, AppError> {
        let binary = PathBuf::from(std::env::var("CONFIGURATION_CRASH_BINARY").unwrap());
        let checksum = hex::encode(sha2::Sha256::digest(fs::read(&binary).unwrap()));
        let process = SingBoxProcess::start(
            &binary,
            &checksum,
            &root.join("runtime"),
            configuration,
            mode,
            &std::collections::HashMap::from([(configuration.profiles[0].id.clone(), secret)]),
        )?;
        let pid = process.process_id();
        let path = root.join("core-pids.json");
        let mut observed: Vec<(u32, u16, u16)> = if path.exists() {
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap()
        } else {
            Vec::new()
        };
        observed.push((pid, process.proxy_port, process.control_port));
        resources::persist(&path, &serde_json::to_vec(&observed).unwrap());
        self.0.lock().unwrap().push(process);
        Ok(pid)
    }
    pub(super) fn confirm(&self, previous: Option<&BackendSession>) {
        if let Some(previous) = previous {
            let mut cores = self.0.lock().unwrap();
            if let Some(index) = cores
                .iter()
                .position(|core| core.process_id() == previous.process_id)
            {
                cores.remove(index); // Drop kills and waits for exactly the old core.
            }
        }
    }
}

pub(super) fn prepare_binary(directory: &std::path::Path) -> PathBuf {
    let binary = directory.join("fake-sing-box.exe");
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-sing-box.rs");
    assert!(Command::new("rustc")
        .arg("--edition=2021")
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap()
        .success());
    binary
}

pub(super) fn checkpoint_ready() {
    let root = PathBuf::from(std::env::var("CONFIGURATION_CRASH_ROOT").unwrap());
    resources::persist(&root.join("crash-ready"), b"");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("crash-release").exists() {
        assert!(
            Instant::now() < deadline,
            "parent did not release crash checkpoint"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) struct ObservedCores {
    handles: Vec<HANDLE>,
    ports: Vec<u16>,
}
impl Drop for ObservedCores {
    fn drop(&mut self) {
        for handle in &self.handles {
            unsafe { CloseHandle(*handle) };
        }
    }
}
impl ObservedCores {
    pub(super) fn capture(root: &std::path::Path, owner: &mut std::process::Child) -> Self {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !root.join("crash-ready").exists() {
            assert!(Instant::now() < deadline, "owner did not reach checkpoint");
            assert!(
                owner.try_wait().unwrap().is_none(),
                "owner exited before checkpoint"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let processes: Vec<(u32, u16, u16)> =
            serde_json::from_slice(&fs::read(root.join("core-pids.json")).unwrap()).unwrap();
        let mut observed = Self {
            handles: vec![],
            ports: vec![],
        };
        for (pid, proxy, control) in processes {
            // Capture live handles before owner exit, preventing PID reuse from
            // being mistaken for successful crash cleanup.
            let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) }; // SYNCHRONIZE
            if !handle.is_null() {
                observed.handles.push(handle);
            }
            observed.ports.extend([proxy, control]);
        }
        assert!(
            !observed.handles.is_empty(),
            "no actual protected core was running"
        );
        resources::persist(&root.join("crash-release"), b"");
        observed
    }
    pub(super) fn assert_exited(&self, root: &std::path::Path) {
        for handle in &self.handles {
            assert_eq!(unsafe { WaitForSingleObject(*handle, 10_000) }, 0);
        }
        for port in &self.ports {
            assert!(
                TcpListener::bind(("127.0.0.1", *port)).is_ok(),
                "core port {port} survived crash"
            );
        }
        crate::sing_box_process::cleanup_stale_runtime_dirs(&root.join("runtime")).unwrap();
        assert_eq!(fs::read_dir(root.join("runtime")).unwrap().count(), 0);
    }
}
