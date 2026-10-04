use crate::{error::AppError, models::PersistedConfiguration};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Exact registry value, including its type. None represents an absent value.
/// Internal only: never include it in IPC responses or diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupEntry {
    pub value_type: u32,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupChange {
    pub original: Option<StartupEntry>,
    pub expected: Option<StartupEntry>,
}

/// Stores only configurations, opaque keyring references and side-effect
/// ownership evidence. Secret values have no representable field here.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryIntent {
    pub transaction_id: String,
    pub previous_revision: u64,
    pub next_revision: u64,
    pub previous: PersistedConfiguration,
    pub candidate: PersistedConfiguration,
    pub staged_refs: Vec<String>,
    pub retired_refs: Vec<String>,
    pub startup: Option<StartupChange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRecord {
    pub intent: RecoveryIntent,
    pub committed: bool,
}

impl RecoveryIntent {
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        self.previous.validate()?;
        self.candidate.validate()?;
        if uuid::Uuid::parse_str(&self.transaction_id).is_err()
            || self.previous_revision.checked_add(1) != Some(self.next_revision)
        {
            return Err(recovery_error());
        }
        let previous_refs: HashSet<_> = self
            .previous
            .profiles
            .iter()
            .filter_map(|p| p.credential_ref.as_deref())
            .collect();
        let candidate_refs: HashSet<_> = self
            .candidate
            .profiles
            .iter()
            .filter_map(|p| p.credential_ref.as_deref())
            .collect();
        let staged: HashSet<_> = self.staged_refs.iter().map(String::as_str).collect();
        let retired: HashSet<_> = self.retired_refs.iter().map(String::as_str).collect();
        // Clean-up cannot remove a reference used by the selected committed
        // side. All referenced keys must be accounted for before any writes.
        if staged.len() != self.staged_refs.len()
            || retired.len() != self.retired_refs.len()
            || staged
                .iter()
                .any(|key| previous_refs.contains(key) || !candidate_refs.contains(key))
            || retired
                .iter()
                .any(|key| candidate_refs.contains(key) || !previous_refs.contains(key))
            || candidate_refs
                .difference(&previous_refs)
                .any(|key| !staged.contains(key))
            || previous_refs
                .difference(&candidate_refs)
                .any(|key| !retired.contains(key))
        {
            return Err(recovery_error());
        }
        Ok(())
    }
}

pub(crate) fn recovery_error() -> AppError {
    AppError {
        code: "configuration_recovery".into(),
        message: "配置恢复记录不一致，请保留数据并检查恢复状态".into(),
        fields: Vec::new(),
        context: None,
    }
}
