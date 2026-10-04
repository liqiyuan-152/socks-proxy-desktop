use crate::{
    configuration_recovery::recovery_error, credentials::CredentialStore, error::AppError,
    runtime_session::SessionLease, startup::StartupAdapter, store::ConfigurationStore,
};

/// Call only after system-proxy recovery and stale-process cleanup succeeded.
/// Holding the lease prevents another application instance from taking over
/// while the committed configuration's external resources are reconciled.
pub(crate) fn recover_configuration_on_startup(
    _lease: &SessionLease,
    store: &dyn ConfigurationStore,
    credentials: &dyn CredentialStore,
    startup: &dyn StartupAdapter,
) -> Result<(), AppError> {
    recover_configuration_after_network_recovery(store, credentials, startup)
}

/// Only after stopping the live runtime and recovering its system proxy.
pub(crate) fn recover_configuration_after_network_recovery(
    store: &dyn ConfigurationStore,
    credentials: &dyn CredentialStore,
    startup: &dyn StartupAdapter,
) -> Result<(), AppError> {
    let Some(record) = store.recovery_record()? else {
        return Ok(());
    };
    let intent = &record.intent;
    intent.validate()?;
    let (configuration, revision, cleanup) = if record.committed {
        (
            &intent.candidate,
            intent.next_revision,
            &intent.retired_refs,
        )
    } else {
        (
            &intent.previous,
            intent.previous_revision,
            &intent.staged_refs,
        )
    };
    if store.load()? != *configuration || store.recovery_revision()? != revision {
        return Err(recovery_error());
    }
    // Do not discard old entries if a committed replacement is missing.
    if record.committed {
        for reference in &intent.staged_refs {
            if credentials.get(reference)?.is_none() {
                return Err(recovery_error());
            }
        }
    }
    if let Some(change) = &intent.startup {
        let (previous, next) = if record.committed {
            (change.original.as_ref(), change.expected.as_ref())
        } else {
            (change.expected.as_ref(), change.original.as_ref())
        };
        startup.compare_exchange_entry(previous, next)?;
    }
    let mut failed = false;
    for reference in cleanup {
        failed |= credentials.delete(reference).is_err();
    }
    if failed {
        return Err(recovery_error());
    }
    store.clear_recovery(&intent.transaction_id)
}

#[cfg(test)]
#[path = "configuration_startup_recovery_tests.rs"]
mod tests;

#[cfg(all(test, windows))]
#[path = "configuration_windows_acceptance.rs"]
mod windows_acceptance;
