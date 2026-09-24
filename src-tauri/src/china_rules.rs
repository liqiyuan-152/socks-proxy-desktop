use crate::error::AppError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const CORE_SHA256: &str = "b838de45bd0b2e6ddbed1977e4745622f7dffab3b293807ff4c6b1b640fed909";

const MANIFEST: &str = include_str!("../resources/china-rules/manifest.json");
const FILES: [(&str, &str); 3] = [
    ("china-domains.srs", "cn-domain"),
    ("china-ipv4.srs", "cn-v4"),
    ("china-ipv6.srs", "cn-v6"),
];

#[derive(Deserialize)]
struct Manifest {
    core_version: String,
    data_date: String,
    hashes: HashMap<String, String>,
}

pub struct ChinaRuleSets {
    root: PathBuf,
    data_date: String,
}

impl ChinaRuleSets {
    pub fn verify(root: &Path) -> Result<Self, AppError> {
        let manifest: Manifest = serde_json::from_str(MANIFEST).map_err(|_| invalid_rules())?;
        if manifest.core_version != "1.14.1" {
            return Err(invalid_rules());
        }
        for (name, _) in FILES {
            let expected = manifest.hashes.get(name).ok_or_else(invalid_rules)?;
            let contents = fs::read(root.join(name)).map_err(|_| invalid_rules())?;
            if hex::encode(Sha256::digest(contents)) != *expected {
                return Err(invalid_rules());
            }
        }
        Ok(Self {
            root: root.into(),
            data_date: manifest.data_date,
        })
    }

    pub fn rule_sets(&self) -> Vec<serde_json::Value> {
        FILES
            .iter()
            .map(|(name, tag)| {
                serde_json::json!({
                    "type": "local", "tag": tag, "format": "binary", "path": self.root.join(name)
                })
            })
            .collect()
    }

    pub fn data_date(&self) -> &str {
        &self.data_date
    }

    pub fn matches(&self, name: &str, target: &str) -> Result<bool, AppError> {
        if !FILES.iter().any(|(filename, _)| *filename == name) {
            return Err(invalid_rules());
        }
        let core = self
            .root
            .parent()
            .ok_or_else(invalid_rules)?
            .join("sing-box/windows-amd64/sing-box.exe");
        let bytes = fs::read(&core).map_err(|_| invalid_rules())?;
        if hex::encode(Sha256::digest(bytes)) != CORE_SHA256 {
            return Err(invalid_rules());
        }
        let output = Command::new(core)
            .args(["rule-set", "match"])
            .arg(self.root.join(name))
            .args([target, "-f", "binary", "--disable-color"])
            .output()
            .map_err(|_| invalid_rules())?;
        if !output.status.success() {
            return Err(invalid_rules());
        }
        let result = String::from_utf8(output.stderr).map_err(|_| invalid_rules())?;
        Ok(result.starts_with("match rules."))
    }
}

pub fn resource_root(binary: &Path) -> Result<PathBuf, AppError> {
    binary
        .ancestors()
        .nth(3)
        .map(|path| path.join("china-rules"))
        .ok_or_else(invalid_rules)
}

fn invalid_rules() -> AppError {
    AppError::unavailable("国内直连规则集缺失、损坏或与内核版本不兼容")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_bundled_data_and_rejects_missing_or_corrupt_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
        let sets = ChinaRuleSets::verify(&root).unwrap();
        assert_eq!(sets.rule_sets().len(), 3);
        let temp = tempfile::tempdir().unwrap();
        assert!(ChinaRuleSets::verify(temp.path()).is_err());
        for (name, _) in FILES {
            fs::copy(root.join(name), temp.path().join(name)).unwrap();
        }
        assert!(ChinaRuleSets::verify(temp.path()).is_ok());
        fs::write(temp.path().join(FILES[0].0), b"corrupt").unwrap();
        assert!(ChinaRuleSets::verify(temp.path()).is_err());
    }
}
