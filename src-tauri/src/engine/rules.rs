use crate::engine::types::PathZone;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::path::Path;

const DEFAULT_IGNORE: &[&str] = &[
    "**/node_modules/**",
    "**/dist/**",
    "**/build/**",
    "**/target/**",
    "**/.git/**",
    "**/.svn/**",
    "**/.hg/**",
    "**/__pycache__/**",
    "**/.venv/**",
    "**/venv/**",
    "**/.idea/**",
    "**/.vscode/**",
    "**/bin/**",
    "**/obj/**",
    "**/.next/**",
    "**/.turbo/**",
    "**/coverage/**",
    "**/*.exe",
    "**/*.dll",
    "**/*.pdb",
    "**/.DS_Store",
    "**/Thumbs.db",
];

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
}

impl SyncRules {
    pub fn load(project_root: &Path) -> Self {
        let mut ignore_patterns: Vec<String> =
            DEFAULT_IGNORE.iter().map(|s| (*s).to_string()).collect();
        let mut intranet: Vec<String> = Vec::new();
        let mut internet: Vec<String> = Vec::new();

        let config_path = project_root.join("codesetu.toml");
        if let Ok(raw) = std::fs::read_to_string(&config_path) {
            if let Ok(parsed) = toml::from_str::<RulesFile>(&raw) {
                ignore_patterns.extend(parsed.ignore);
                intranet = parsed.intranet_only.paths;
                internet = parsed.internet_only.paths;
            }
        }

        let secrets: Vec<String> = DEFAULT_SECRET.iter().map(|s| (*s).to_string()).collect();

        Self {
            ignore: build_set(&ignore_patterns),
            intranet_only: build_set(&intranet),
            internet_only: build_set(&internet),
            secrets: build_set(&secrets),
        }
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
        // Fast path for common build dirs
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
        ) || self.ignore.is_match(&with_slash)
    }
}

fn build_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            builder.add(g);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
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
        let rules = SyncRules::load(dir.path());
        assert_eq!(rules.zone("svc/node_modules/x.js"), PathZone::Ignored);
        assert_eq!(rules.zone("src/main.rs"), PathZone::Shared);
        assert_eq!(rules.zone(".env"), PathZone::ParkedSecret);
    }

    #[test]
    fn loads_codesetu_toml_zones() {
        let dir = tempdir().unwrap();
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
    }
}
