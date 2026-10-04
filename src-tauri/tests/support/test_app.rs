use super::{backend::MockBackend, credentials::InMemoryCredentialStore};
use socks_proxy_lib::{
    services::{
        adapters::{
            ConfigurationStore, CredentialUpdate, ProxyProtocol, SqliteConfigurationStore,
            StartupAdapter,
        },
        AppError, ApplicationService, ProfileInput, ProfileView, RuntimeMode,
    },
    ManagedRuntime,
};
use std::path::PathBuf;
use tempfile::TempDir;

struct MockStartup;
impl StartupAdapter for MockStartup {
    fn is_enabled(&self) -> Result<bool, AppError> {
        Ok(false)
    }
}

pub struct TestApp {
    pub service: ApplicationService,
    pub backend: MockBackend,
    database: PathBuf,
    _directory: TempDir,
}
impl TestApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::open(tempfile::tempdir()?)
    }
    fn open(directory: TempDir) -> Result<Self, Box<dyn std::error::Error>> {
        let database = directory.path().join("configuration.sqlite");
        let store = SqliteConfigurationStore::open(&database)?;
        let backend = MockBackend::default();
        let runtime = ManagedRuntime::new(
            store.load()?,
            Box::new(backend.clone()),
            &uuid::Uuid::new_v4().to_string(),
            RuntimeMode::Direct,
        )?;
        let service = ApplicationService::new(
            Box::new(store),
            Box::new(InMemoryCredentialStore::default()),
            Box::new(MockStartup),
            Box::new(runtime),
        );
        Ok(Self {
            service,
            backend,
            database,
            _directory: directory,
        })
    }
    pub fn database(&self) -> &std::path::Path {
        &self.database
    }
    pub fn restart(self) -> Result<Self, Box<dyn std::error::Error>> {
        let Self {
            service,
            _directory,
            ..
        } = self;
        drop(service);
        Self::open(_directory)
    }
    pub fn add_profile(&self, name: &str) -> Result<ProfileView, AppError> {
        self.service
            .save_profile(Self::profile_input(None, name, CredentialUpdate::Delete))
    }
    pub fn profile_input(
        id: Option<String>,
        name: &str,
        credential: CredentialUpdate,
    ) -> ProfileInput {
        ProfileInput {
            id,
            name: name.into(),
            protocol: ProxyProtocol::Socks5,
            host: "127.0.0.1".into(),
            port: 1080,
            authentication_enabled: !matches!(credential, CredentialUpdate::Delete),
            enabled: true,
            credential: Some(credential),
        }
    }
    pub fn persisted(
        &self,
    ) -> Result<socks_proxy_lib::services::adapters::PersistedConfiguration, AppError> {
        SqliteConfigurationStore::open(&self.database)?.load()
    }
}
