use crate::{
    configuration_startup_recovery::recover_configuration_on_startup,
    credentials::{CredentialStore, CredentialUpdate},
    error::AppError,
    models::{PersistedConfiguration, ProxyProfile, ProxyProtocol, RuntimeMode},
    runtime::{BackendSession, ManagedRuntime, RuntimeBackend, RuntimeCoordinator},
    runtime_session::SessionLease,
    services::ApplicationService,
    startup::StartupAdapter,
    store::{ConfigurationStore, SqliteConfigurationStore},
};
use std::{fs, path::PathBuf, process::Command};

#[path = "configuration_crash_resources.rs"]
mod resources;
use resources::{Credentials, Startup};

#[path = "configuration_upgrade_tests.rs"]
mod upgrade;

#[cfg(windows)]
#[path = "configuration_crash_windows.rs"]
mod windows;

pub(crate) fn checkpoint(stage: &str) {
    if std::env::var("CONFIGURATION_CRASH_STAGE").as_deref() == Ok(stage) {
        #[cfg(windows)]
        windows::checkpoint_ready();
        std::process::exit(86);
    }
}

struct Backend {
    root: PathBuf,
    credentials: Credentials,
    #[cfg(windows)]
    cores: windows::Cores,
}
impl RuntimeBackend for Backend {
    fn transition(
        &self,
        _: Option<&BackendSession>,
        config: &PersistedConfiguration,
        mode: RuntimeMode,
        revision: u64,
    ) -> Result<Option<BackendSession>, AppError> {
        if mode == RuntimeMode::Direct {
            return Ok(None);
        }
        let profile = &config.profiles[0];
        let secret =
            crate::credentials::read_profile_credential(&self.credentials, profile)?.unwrap();
        #[cfg(windows)]
        let process_id = self.cores.start(
            &self.root,
            config,
            mode,
            crate::credentials::ProxyCredential {
                username: secret.username.clone(),
                password: secret.password.clone(),
            },
        )?;
        #[cfg(not(windows))]
        let process_id = 1;
        let session = BackendSession {
            run_id: format!("run-{revision}"),
            process_id,
            configuration_revision: revision,
            system_proxy_enabled: true,
            tun_enabled: false,
        };
        resources::persist(
            &self.root.join("candidate-run.json"),
            &serde_json::to_vec(&(
                profile.credential_ref.clone(),
                secret.username,
                secret.password,
            ))
            .unwrap(),
        );
        Ok(Some(session))
    }
    fn confirm_transition(&self, previous: Option<&BackendSession>) {
        #[cfg(windows)]
        self.cores.confirm(previous);
        #[cfg(not(windows))]
        let _ = previous;
    }
    fn revert_transition(
        &self,
        _: Option<&BackendSession>,
        _: Option<&BackendSession>,
    ) -> Result<(), AppError> {
        Ok(())
    }
    fn reconcile_session(&self, _: &BackendSession) -> Result<bool, AppError> {
        Ok(true)
    }
}

#[test]
#[ignore = "isolated process child invoked by cross-resource crash test"]
fn interrupted_configuration_child() {
    let root = PathBuf::from(std::env::var("CONFIGURATION_CRASH_ROOT").unwrap());
    let lease =
        SessionLease::acquire(&std::env::var("CONFIGURATION_CRASH_LEASE").unwrap()).unwrap();
    let store =
        std::sync::Arc::new(SqliteConfigurationStore::open(root.join("config.sqlite3")).unwrap());
    let current = store.load().unwrap();
    let runtime = ManagedRuntime::from_lease(
        current.clone(),
        Box::new(Backend {
            root: root.clone(),
            credentials: Credentials(root.join("keyring")),
            #[cfg(windows)]
            cores: windows::Cores::default(),
        }),
        lease,
        RuntimeMode::Global,
    )
    .unwrap()
    .with_configuration_revision(store.recovery_revision().unwrap());
    runtime.request_mode(RuntimeMode::Global).unwrap();
    let service = ApplicationService::new(
        Box::new(store),
        Box::new(Credentials(root.join("keyring"))),
        Box::new(Startup(root.join("startup.json"))),
        Box::new(runtime),
    );
    // Import stages new credentials, startup and a real coordinator transition
    // through the same production commit path in one transaction.
    let mut portable: serde_json::Value = serde_json::from_str(&service.export().unwrap()).unwrap();
    portable["settings"]["launch_at_login"] = true.into();
    service
        .import(
            &portable.to_string(),
            std::collections::HashMap::from([(
                "profile".into(),
                CredentialUpdate::Replace {
                    username: "new-user".into(),
                    password: "new-secret".into(),
                },
            )]),
        )
        .unwrap();
    panic!("configured interruption point was not reached");
}

#[test]
fn interrupted_cross_resource_commit_recovers_exact_durable_boundary() {
    #[cfg(windows)]
    let binary_directory = tempfile::tempdir().unwrap();
    #[cfg(windows)]
    let binary = windows::prepare_binary(binary_directory.path());
    for stage in [
        "intent",
        "credentials",
        "startup",
        "runtime",
        "commit",
        "cleanup",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let lease_name = uuid::Uuid::new_v4().to_string();
        let credentials = Credentials(root.join("keyring"));
        credentials
            .replace("profile", "old-user", "old-secret")
            .unwrap();
        let startup = Startup(root.join("startup.json"));
        let previous = PersistedConfiguration {
            profiles: vec![ProxyProfile {
                id: "profile".into(),
                name: "Primary".into(),
                protocol: ProxyProtocol::Socks5,
                host: "proxy.example.com".into(),
                port: 1080,
                authentication_enabled: true,
                credential_ref: Some("profile".into()),
                enabled: true,
            }],
            active_profile_id: Some("profile".into()),
            ..Default::default()
        };
        let store = SqliteConfigurationStore::open(root.join("config.sqlite3")).unwrap();
        store.save(&previous).unwrap();
        drop(store);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "configuration_crash_tests::interrupted_configuration_child",
                "--ignored",
            ])
            .env("CONFIGURATION_CRASH_ROOT", root)
            .env("CONFIGURATION_CRASH_LEASE", &lease_name)
            .env("CONFIGURATION_CRASH_STAGE", stage)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        command.env("CONFIGURATION_CRASH_BINARY", &binary);
        #[cfg(windows)]
        let mut child = command.spawn().unwrap();
        #[cfg(not(windows))]
        let child = command.spawn().unwrap();
        #[cfg(windows)]
        let cores = windows::ObservedCores::capture(root, &mut child);
        let output = child.wait_with_output().unwrap();
        #[cfg(windows)]
        cores.assert_exited(root);
        assert_eq!(output.status.code(), Some(86), "stage {stage}: {output:?}");
        // Direct exit bypassed Rust Drop; reacquiring proves the crashed
        // configuration writer no longer owns the session lease.
        let lease = SessionLease::acquire(&lease_name).unwrap();
        let store = SqliteConfigurationStore::open(root.join("config.sqlite3")).unwrap();
        let record = store.recovery_record().unwrap().unwrap();
        let committed = matches!(stage, "commit" | "cleanup");
        assert_eq!(record.committed, committed);
        assert!(
            credentials.get("profile").unwrap().is_some(),
            "old secret prematurely lost at {stage}"
        );
        let next = &record.intent.staged_refs[0];
        assert_eq!(credentials.get(next).unwrap().is_some(), stage != "intent");
        assert_eq!(
            startup.is_enabled().unwrap(),
            matches!(stage, "startup" | "runtime" | "commit" | "cleanup")
        );
        let observed: (Option<String>, String, String) =
            serde_json::from_slice(&fs::read(root.join("candidate-run.json")).unwrap()).unwrap();
        assert_eq!(
            observed.2,
            if matches!(stage, "runtime" | "commit" | "cleanup") {
                "new-secret"
            } else {
                "old-secret"
            }
        );
        let json = serde_json::to_string(&record.intent).unwrap();
        for secret in ["old-user", "old-secret", "new-user", "new-secret"] {
            assert!(!json.contains(secret));
        }
        recover_configuration_on_startup(&lease, &store, &credentials, &startup).unwrap();
        recover_configuration_on_startup(&lease, &store, &credentials, &startup).unwrap();
        assert_eq!(
            store.load().unwrap(),
            if committed {
                record.intent.candidate.clone()
            } else {
                previous
            }
        );
        assert_eq!(
            store.recovery_revision().unwrap(),
            if committed { 2 } else { 1 }
        );
        assert_eq!(startup.is_enabled().unwrap(), committed);
        assert_eq!(credentials.get("profile").unwrap().is_some(), !committed);
        assert_eq!(credentials.get(next).unwrap().is_some(), committed);
        assert_eq!(fs::read_dir(root.join("keyring")).unwrap().count(), 1);
        assert!(store.recovery_record().unwrap().is_none());
    }
}
