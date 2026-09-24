use super::ConfigurationService;
use crate::{
    china_rules::ChinaRuleSets,
    error::AppError,
    route_test::{self, RouteTestResult},
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct ChinaDirectStatus {
    pub enabled: bool,
    pub available: bool,
    pub data_date: Option<String>,
}

impl ConfigurationService {
    #[cfg(any(windows, test))]
    pub fn with_china_rule_root(mut self, root: PathBuf) -> Self {
        self.china_rule_root = Some(root);
        self
    }

    pub fn china_direct_status(&self) -> Result<ChinaDirectStatus, AppError> {
        let configuration = self.store.load()?;
        let sets = self
            .china_rule_root
            .as_deref()
            .and_then(|root| ChinaRuleSets::verify(root).ok());
        Ok(ChinaDirectStatus {
            enabled: configuration.china_direct_enabled,
            available: sets.is_some(),
            data_date: sets.map(|sets| sets.data_date().to_owned()),
        })
    }

    pub fn test_route(&self, target: &str, port: u16) -> Result<RouteTestResult, AppError> {
        route_test::evaluate(
            &self.store.load()?,
            self.china_rule_root.as_deref(),
            target,
            port,
        )
    }

    pub fn set_china_direct_enabled(&self, enabled: bool) -> Result<ChinaDirectStatus, AppError> {
        let _guard = self.lock()?;
        let current = self.store.load()?;
        let mut candidate = current.clone();
        candidate.china_direct_enabled = enabled;
        self.commit(&current, &candidate)?;
        self.china_direct_status()
    }
}
