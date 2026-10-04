use super::*;
use crate::{
    configuration_recovery::{recovery_error, RecoveryIntent, StartupChange},
    runtime::RuntimeSnapshot,
};
use std::collections::HashSet;

impl ConfigurationContext {
    #[tracing::instrument(skip_all, level = "debug")]
    pub(super) fn commit_durable(
        &self,
        current: &PersistedConfiguration,
        candidate: &PersistedConfiguration,
        staged: Vec<(String, ProxyCredential)>,
        startup: Option<StartupChange>,
    ) -> Result<(), AppError> {
        crate::performance_metrics::measure("configuration_apply", || {
            candidate.validate()?;
            if candidate.china_direct_enabled {
                let root_guard = self
                    .china_rule_root
                    .lock()
                    .map_err(|_| AppError::storage("规则集路径锁不可用"))?;
                let root = root_guard
                    .as_deref()
                    .ok_or_else(|| AppError::unavailable("此平台未提供国内直连规则集"))?;
                ChinaRuleSets::verify(root)?;
            }
            if self.store.recovery_record()?.is_some() {
                return Err(recovery_error());
            }
            let references = |config: &PersistedConfiguration| -> HashSet<String> {
                config
                    .profiles
                    .iter()
                    .filter_map(|p| p.credential_ref.clone())
                    .collect()
            };
            let previous_refs = references(current);
            let candidate_refs = references(candidate);
            let previous_revision = self.store.recovery_revision()?;
            let intent = RecoveryIntent {
                transaction_id: uuid::Uuid::new_v4().to_string(),
                previous_revision,
                next_revision: previous_revision
                    .checked_add(1)
                    .ok_or_else(recovery_error)?,
                previous: current.clone(),
                candidate: candidate.clone(),
                staged_refs: staged
                    .iter()
                    .map(|(reference, _)| reference.clone())
                    .collect(),
                retired_refs: previous_refs.difference(&candidate_refs).cloned().collect(),
                startup,
            };
            let span = tracing::info_span!("configuration_transaction", transaction_id = %intent.transaction_id, revision = intent.next_revision);
            let _entered = span.enter();
            tracing::debug!(stage = "begin", "configuration transaction");
            // The durable intent precedes every keyring/registry/runtime write.
            self.store.begin_recovery(&intent)?;
            #[cfg(test)]
            crate::configuration_crash_tests::checkpoint("intent");
            let snapshot = self.runtime.snapshot();
            let changed_ids: Vec<_> = candidate
                .profiles
                .iter()
                .filter(|p| {
                    current
                        .profiles
                        .iter()
                        .find(|old| old.id == p.id)
                        .map(|old| &old.credential_ref)
                        != Some(&p.credential_ref)
                })
                .map(|p| p.id.clone())
                .collect();
            let mut applied = false;
            let preparation = (|| {
                for (reference, secret) in &staged {
                    self.credentials
                        .replace(reference, &secret.username, &secret.password)?;
                }
                #[cfg(test)]
                crate::configuration_crash_tests::checkpoint("credentials");
                if let Some(change) = &intent.startup {
                    self.startup.compare_exchange_entry(
                        change.original.as_ref(),
                        change.expected.as_ref(),
                    )?;
                }
                #[cfg(test)]
                crate::configuration_crash_tests::checkpoint("startup");
                self.runtime.apply_configuration_with_credentials(
                    current,
                    candidate,
                    &changed_ids,
                )?;
                applied = true;
                #[cfg(test)]
                crate::configuration_crash_tests::checkpoint("runtime");
                self.store.commit_recovery(&intent.transaction_id)?;
                tracing::debug!(stage = "commit", "configuration transaction");
                #[cfg(test)]
                crate::configuration_crash_tests::checkpoint("commit");
                Ok(())
            })();
            if let Err(error) = preparation {
                // Re-read the durable marker before selecting rollback. An
                // uncertain commit must never delete the committed credentials.
                let record = self.store.recovery_record()?.ok_or_else(recovery_error)?;
                if !record.committed {
                    if let Err(error) = self.rollback_durable(&intent, applied.then_some(&snapshot))
                    {
                        self.report_configuration_recovery_issue(&error);
                        return Err(error);
                    }
                    return Err(error);
                }
            }
            self.invalidate_latency_tasks(intent.next_revision);
            self.runtime.confirm_configuration();
            #[cfg(test)]
            crate::configuration_crash_tests::checkpoint("cleanup");
            // A cleanup failure keeps the committed marker: subsequent recovery
            // must finish the new state, never roll back its database or secrets.
            let cleanup = self
                .cleanup_references(&intent.retired_refs)
                .and_then(|_| self.store.clear_recovery(&intent.transaction_id));
            if let Err(error) = &cleanup {
                self.report_configuration_recovery_issue(error);
            }
            tracing::info!(
                stage = "cleanup",
                succeeded = cleanup.is_ok(),
                "configuration transaction"
            );
            cleanup
        })
    }

    fn rollback_durable(
        &self,
        intent: &RecoveryIntent,
        snapshot: Option<&RuntimeSnapshot>,
    ) -> Result<(), AppError> {
        tracing::warn!(stage = "rollback", "configuration transaction");
        let mut failed = false;
        if let Some(snapshot) = snapshot {
            failed |= self
                .runtime
                .restore_configuration(&intent.previous, snapshot)
                .is_err();
        }
        if let Some(change) = &intent.startup {
            failed |= self
                .startup
                .compare_exchange_entry(change.expected.as_ref(), change.original.as_ref())
                .is_err();
        }
        // Attempt all recovery actions even if another resource failed.
        // If a candidate session could not be reverted, retain its credentials
        // until recovery has safely stopped it; do not destroy its live input.
        if !failed {
            failed |= self.cleanup_references(&intent.staged_refs).is_err();
        }
        if failed {
            return Err(rollback_failed());
        }
        self.store.clear_recovery(&intent.transaction_id)
    }

    fn cleanup_references(&self, references: &[String]) -> Result<(), AppError> {
        let mut failed = false;
        for reference in references {
            failed |= self.credentials.delete(reference).is_err();
        }
        if failed {
            Err(recovery_error())
        } else {
            Ok(())
        }
    }
}
