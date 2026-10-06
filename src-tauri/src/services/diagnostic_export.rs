//! 原生选择保存位置后原子写入导出内容，失败时保留原文件。
use super::ApplicationService;
use crate::{error::AppError, error_context::ErrorDomain};
use std::{io::Write, path::Path};

fn write_export(path: &Path, content: &str) -> Result<(), AppError> {
    let failure = || AppError::storage("无法保存导出文件，请检查目录权限和磁盘空间");
    let parent = path.parent().ok_or_else(failure)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| failure())?;
    temporary
        .write_all(content.as_bytes())
        .map_err(|_| failure())?;
    temporary.as_file().sync_all().map_err(|_| failure())?;
    temporary.persist(path).map_err(|_| failure())?;
    Ok(())
}

impl ApplicationService {
    /// 配置内容来自脱敏导出；不接受前端传入文件路径或内容。
    pub fn save_configuration_file(&self, path: &Path, content: &str) -> Result<(), AppError> {
        self.finish(
            write_export(path, content),
            ErrorDomain::Storage,
            "save_configuration",
        )
    }

    /// 仅由原生保存选择器调用；IPC 不接受任意文件路径。
    pub fn save_diagnostics_file(&self, path: &Path, content: &str) -> Result<(), AppError> {
        self.finish(
            write_export(path, content),
            ErrorDomain::Storage,
            "save_runtime_diagnostics",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_file_matches_sanitized_export_and_has_storage_context_on_failure(
    ) -> Result<(), AppError> {
        use crate::services::application_service::tests::{profile_input, Fixture};
        let fixture = Fixture::new();
        fixture.service.save_profile(profile_input())?;
        let content = fixture.service.export()?;
        let directory = tempfile::tempdir().expect("configuration export directory");
        let path = directory.path().join("configuration.json");
        fixture.service.save_configuration_file(&path, &content)?;
        let saved = std::fs::read_to_string(&path).expect("saved configuration");
        assert_eq!(saved, content);
        let parsed: serde_json::Value = serde_json::from_str(&saved).expect("JSON configuration");
        assert_eq!(parsed["schema_version"], 3);
        assert!(!saved.contains("password"));
        let error = fixture
            .service
            .save_configuration_file(
                &directory.path().join("missing").join("configuration.json"),
                &content,
            )
            .expect_err("missing export directory");
        assert_eq!(
            error.context.expect("storage context").operation.as_deref(),
            Some("save_configuration")
        );
        assert_eq!(
            std::fs::read_to_string(path).expect("original configuration"),
            content
        );
        Ok(())
    }

    #[test]
    fn saves_json_lines_and_replaces_existing_file_only_after_write() -> Result<(), AppError> {
        let directory = tempfile::tempdir().expect("export directory");
        let path = directory.path().join("diagnostics.jsonl");
        std::fs::write(&path, "previous export").expect("old export");
        write_export(&path, "{\"id\":\"error-42\"}\n")?;
        assert_eq!(
            std::fs::read_to_string(&path).expect("saved export"),
            "{\"id\":\"error-42\"}\n"
        );
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("export directory")
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn facade_attaches_storage_identity_when_persistence_fails() {
        let fixture = crate::services::application_service::tests::Fixture::new();
        let root = tempfile::tempdir().expect("export directory");
        let target = root.path().join("missing").join("export.jsonl");
        let error = fixture
            .service
            .save_diagnostics_file(&target, "data")
            .expect_err("missing parent");
        let context = error.context.expect("storage context");
        assert_eq!(context.domain, ErrorDomain::Storage);
        assert_eq!(
            context.operation.as_deref(),
            Some("save_runtime_diagnostics")
        );
        assert!(!context.error_id.is_empty());
        let target = root.path().join("export.jsonl");
        fixture
            .service
            .save_diagnostics_file(&target, "{\"id\":\"safe\"}\n")
            .expect("facade save");
        assert_eq!(
            std::fs::read_to_string(target).expect("file"),
            "{\"id\":\"safe\"}\n"
        );
    }

    #[test]
    fn failed_save_preserves_destination_and_hides_local_paths() {
        let directory = tempfile::tempdir().expect("export directory");
        let destination = directory.path().join("private-destination");
        std::fs::create_dir(&destination).expect("directory destination");
        std::fs::write(destination.join("original"), "unchanged").expect("original data");
        let error =
            write_export(&destination, "new export").expect_err("directory cannot be replaced");
        assert_eq!(error.code, "storage_error");
        assert!(!error.message.contains("private-destination"));
        assert_eq!(
            std::fs::read_to_string(destination.join("original")).expect("preserved data"),
            "unchanged"
        );
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("directory")
                .count(),
            1
        );
        assert!(write_export(
            &directory.path().join("missing").join("export.jsonl"),
            "data"
        )
        .is_err());
    }
}
