//! 直接通过领域接口验证迁移后的档案行为。
use super::*;
use crate::{
    credentials::CredentialStore,
    error::AppError,
    services::application_service::tests::{profile_input, Fixture},
    store::ConfigurationStore,
};

#[test]
fn domain_errors_distinguish_missing_references_validation_and_credentials() -> Result<(), AppError>
{
    use crate::domain_errors::CredentialError;
    let fixture = Fixture::new();
    let profiles = ProxyProfileService::new(fixture.service.context.clone());
    assert!(
        matches!(profiles.delete_profile("missing"), Err(ProxyError::NotFound { id }) if id == "missing")
    );
    let mut editing = profile_input();
    editing.id = Some("missing".into());
    assert!(matches!(
        profiles.save_profile(editing),
        Err(ProxyError::NotFound { .. })
    ));
    let mut invalid = profile_input();
    invalid.port = 0;
    assert!(
        matches!(profiles.save_profile(invalid), Err(ProxyError::ValidationFailed(value)) if value.fields[0].field == "profiles[0].port")
    );
    let mut invalid_credential = profile_input();
    invalid_credential.authentication_enabled = true;
    invalid_credential.credential = Some(CredentialUpdate::Replace {
        username: "".into(),
        password: "".into(),
    });
    assert!(matches!(
        profiles.save_profile(invalid_credential),
        Err(ProxyError::CredentialInvalid(CredentialError::Invalid(_)))
    ));
    let profile = profiles.save_profile(profile_input())?;
    assert!(matches!(
        profiles.get_credential(&profile.id),
        Err(ProxyError::CredentialInvalid(CredentialError::Disabled))
    ));
    profiles.select_profile(Some(profile.id.clone()))?;
    assert!(
        matches!(profiles.delete_profile(&profile.id), Err(ProxyError::InUse { references }) if references[0].field == "default_profile_id")
    );
    fixture
        .service
        .context
        .startup_recovery_pending
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(matches!(
        profiles.select_profile(None),
        Err(ProxyError::RecoveryInProgress)
    ));
    Ok(())
}

#[test]
fn credentials_remain_isolated_when_another_profile_commit_fails() -> Result<(), AppError> {
    let fixture = Fixture::new();
    let profiles: Arc<dyn ProfileService> =
        Arc::new(ProxyProfileService::new(fixture.service.context.clone()));
    let mut first = profile_input();
    first.authentication_enabled = true;
    first.credential = Some(CredentialUpdate::Replace {
        username: "first-user".into(),
        password: "first-secret".into(),
    });
    let first = profiles.save_profile(first)?;
    let mut second = profile_input();
    second.name = "Secondary".into();
    second.authentication_enabled = true;
    second.credential = Some(CredentialUpdate::Replace {
        username: "second-user".into(),
        password: "second-secret".into(),
    });
    let second = profiles.save_profile(second)?;
    let before = fixture.store.load()?;
    let first_reference = before.profiles[0].credential_ref.clone();
    let second_reference = before.profiles[1].credential_ref.clone();
    assert_ne!(first_reference, second_reference);
    *fixture.store.fail_save.lock().expect("test store lock") = true;
    let mut update = profile_input();
    update.id = Some(second.id.clone());
    update.name = "Secondary".into();
    update.authentication_enabled = true;
    update.credential = Some(CredentialUpdate::Replace {
        username: "replacement-user".into(),
        password: "replacement-secret".into(),
    });
    assert!(profiles.save_profile(update).is_err());
    let after = fixture.store.load()?;
    assert_eq!(after.profiles[0].credential_ref, first_reference);
    assert_eq!(after.profiles[1].credential_ref, second_reference);
    assert_eq!(profiles.get_credential(&first.id)?.password, "first-secret");
    assert_eq!(
        profiles.get_credential(&second.id)?.password,
        "second-secret"
    );
    assert_eq!(
        fixture.store.recovery_revision()?,
        second.configuration_revision
    );
    assert!(fixture.store.recovery_record()?.is_none());
    assert!(!fixture.service.export()?.contains("secret"));
    Ok(())
}

#[test]
fn credentials_are_read_only_for_existing_authenticated_profiles() {
    let fixture = Fixture::new();
    let profiles: Arc<dyn ProfileService> =
        Arc::new(ProxyProfileService::new(fixture.service.context.clone()));
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "secret".into(),
    });
    let created = profiles.save_profile(input).unwrap();
    let credential = profiles.get_credential(&created.id).unwrap();
    assert_eq!(credential.username, "alice");
    assert_eq!(credential.password, "secret");
    assert_eq!(profiles.list_profiles().unwrap()[0].name, "Primary");
    assert_eq!(
        AppError::from(profiles.get_credential("missing").err().unwrap()).code,
        "proxy_not_found"
    );

    let mut plain = profile_input();
    plain.name = "Plain".into();
    let plain = profiles.save_profile(plain).unwrap();
    assert_eq!(
        AppError::from(profiles.get_credential(&plain.id).err().unwrap()).code,
        "credential_error"
    );
    let reference = fixture.store.load().unwrap().profiles[0]
        .credential_ref
        .clone()
        .unwrap();
    fixture.credentials.delete(&reference).unwrap();
    assert_eq!(
        AppError::from(profiles.get_credential(&created.id).err().unwrap()).code,
        "credential_error"
    );
}

#[test]
fn validates_profile_inputs_before_writing_and_returns_field_errors() {
    let fixture = Fixture::new();
    let profiles: Arc<dyn ProfileService> =
        Arc::new(ProxyProfileService::new(fixture.service.context.clone()));
    let created = profiles.save_profile(profile_input()).unwrap();
    assert!(!created.id.is_empty());
    let mut duplicate = profile_input();
    duplicate.name = "primary".into();
    assert_eq!(
        AppError::from(profiles.save_profile(duplicate).unwrap_err()).fields[0].field,
        "profiles[1].name"
    );
    let mut invalid = profile_input();
    invalid.name = "Secondary".into();
    invalid.port = 0;
    assert_eq!(
        AppError::from(profiles.save_profile(invalid).unwrap_err()).fields[0].field,
        "profiles[1].port"
    );
    assert_eq!(profiles.list_profiles().unwrap().len(), 1);
}

#[test]
fn deleting_active_profile_waits_for_runtime_and_rolls_back_secret_on_failure() {
    let fixture = Fixture::new();
    let profiles: Arc<dyn ProfileService> =
        Arc::new(ProxyProfileService::new(fixture.service.context.clone()));
    let mut input = profile_input();
    input.authentication_enabled = true;
    input.credential = Some(CredentialUpdate::Replace {
        username: "alice".into(),
        password: "secret-value".into(),
    });
    let created = profiles.save_profile(input).unwrap();
    profiles.select_profile(Some(created.id.clone())).unwrap();
    assert_eq!(
        AppError::from(profiles.delete_profile(&created.id).unwrap_err()).fields[0].field,
        "default_profile_id"
    );
    profiles.select_profile(None).unwrap();
    *fixture.runtime.reject_next.lock().unwrap() = true;
    assert!(profiles.delete_profile(&created.id).is_err());
    assert_eq!(fixture.store.load().unwrap().active_profile_id, None);
    assert_eq!(
        profiles.get_credential(&created.id).unwrap().password,
        "secret-value"
    );
    let reference = fixture.store.load().unwrap().profiles[0]
        .credential_ref
        .clone()
        .unwrap();
    profiles.delete_profile(&created.id).unwrap();
    assert_eq!(fixture.store.load().unwrap().active_profile_id, None);
    assert!(fixture.credentials.get(&reference).unwrap().is_none());
    assert_eq!(
        fixture.runtime.committed_active_ids.lock().unwrap().last(),
        Some(&None)
    );
}
