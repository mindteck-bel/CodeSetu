use crate::engine::ignore_config::{
    build_globset, effective_config, effective_pattern_strings, IgnoreConfigFile,
};
use crate::engine::types::PathZone;
use globset::GlobSet;
use serde::Deserialize;
use std::path::Path;

const DEFAULT_SECRET: &[&str] = &["**/.env", "**/.env.*", "**/*.pem", "**/*.key"];

#[derive(Debug, Deserialize, Default)]
struct RulesFile {
    #[serde(default)]
    ignore: Vec<String>,
    #[serde(default, rename = "intranet-only")]
    intranet_only: ZoneSection,
    #[serde(default, rename = "internet-only")]
    internet_only: ZoneSection,
}

#[derive(Debug, Deserialize, Default)]
struct ZoneSection {
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Debug)]
pub struct SyncRules {
    ignore: GlobSet,
    intranet_only: GlobSet,
    internet_only: GlobSet,
    secrets: GlobSet,
    pub warnings: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

impl SyncRules {
    pub fn load(project_root: &Path) -> Self {
        let (ignore_cfg, mut warnings) = effective_config(Some(project_root));
        let ignore_patterns = effective_pattern_strings(&ignore_cfg);

        let mut intranet: Vec<String> = Vec::new();
        let mut internet: Vec<String> = Vec::new();

        let config_path = project_root.join("codesetu.toml");
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(raw) => match toml::from_str::<RulesFile>(&raw) {
                    Ok(parsed) => {
                        // ignore list already merged via effective_config
                        let _ = parsed.ignore;
                        intranet = parsed.intranet_only.paths;
                        internet = parsed.internet_only.paths;
                    }
                    Err(e) => warnings.push(format!(
                        "codesetu.toml parse failed — using defaults for zones: {e}"
                    )),
                },
                Err(e) => warnings.push(format!("Could not read codesetu.toml: {e}")),
            }
        }

        let secrets: Vec<String> = DEFAULT_SECRET.iter().map(|s| (*s).to_string()).collect();

        Self {
            ignore: build_globset(&ignore_patterns),
            intranet_only: build_globset(&intranet),
            internet_only: build_globset(&internet),
            secrets: build_globset(&secrets),
            warnings,
            ignore_patterns,
        }
    }

    /// Build from an explicit ignore config (tests / preview).
    pub fn from_ignore_config(cfg: &IgnoreConfigFile, project_root: Option<&Path>) -> Self {
        let mut base = if let Some(root) = project_root {
            Self::load(root)
        } else {
            let patterns = effective_pattern_strings(cfg);
            Self {
                ignore: build_globset(&patterns),
                intranet_only: GlobSet::empty(),
                internet_only: GlobSet::empty(),
                secrets: build_globset(
                    &DEFAULT_SECRET
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect::<Vec<_>>(),
                ),
                warnings: vec![],
                ignore_patterns: patterns,
            }
        };
        let patterns = effective_pattern_strings(cfg);
        base.ignore = build_globset(&patterns);
        base.ignore_patterns = patterns;
        base
    }

    pub fn zone(&self, rel: &str) -> PathZone {
        let normalized = normalize_rel(rel);
        if self.ignore.is_match(&normalized) {
            return PathZone::Ignored;
        }
        if self.secrets.is_match(&normalized) {
            return PathZone::ParkedSecret;
        }
        if self.intranet_only.is_match(&normalized) {
            return PathZone::IntranetOnly;
        }
        if self.internet_only.is_match(&normalized) {
            return PathZone::InternetOnly;
        }
        PathZone::Shared
    }

    pub fn should_skip_dir(&self, rel_dir: &str) -> bool {
        let with_slash = if rel_dir.is_empty() {
            String::new()
        } else {
            format!("{}/", normalize_rel(rel_dir))
        };
        if with_slash.is_empty() {
            return false;
        }
        let name = Path::new(rel_dir)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        matches!(
            name,
            "node_modules"
                | "dist"
                | "build"
                | "target"
                | ".git"
                | "__pycache__"
                | ".venv"
                | "venv"
                | ".next"
                | ".turbo"
                | "coverage"
                | "bin"
                | "obj"
                | ".cache"
                | "tmp"
                | ".svn"
                | ".hg"
                | ".idea"
                | ".vscode"
        ) || self.ignore.is_match(&with_slash)
            || self.ignore.is_match(&normalize_rel(rel_dir))
    }
}

fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_ignores_node_modules() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            let rules = SyncRules::load(dir.path());
            assert_eq!(rules.zone("svc/node_modules/x.js"), PathZone::Ignored);
            assert_eq!(rules.zone("src/main.rs"), PathZone::Shared);
            assert_eq!(rules.zone(".env"), PathZone::ParkedSecret);
        });
    }

    #[test]
    fn loads_codesetu_toml_zones() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(&dir.path().join("data"), || {
            fs::write(
                dir.path().join("codesetu.toml"),
                r#"
    ignore = ["**/tmp/**"]
    [intranet-only]
    paths = ["deploy/intra/**"]
    [internet-only]
    paths = ["deploy/cloud/**"]
    "#,
            )
            .unwrap();
            let rules = SyncRules::load(dir.path());
            assert_eq!(rules.zone("deploy/intra/a.yml"), PathZone::IntranetOnly);
            assert_eq!(rules.zone("deploy/cloud/b.yml"), PathZone::InternetOnly);
            assert_eq!(rules.zone("tmp/x"), PathZone::Ignored);
        });
    }
}
