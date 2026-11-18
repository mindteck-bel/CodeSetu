//! Ignore Rules Manager — versioned offline config with categories and precedence.
//!
//! Precedence (highest wins for enable/disable of a pattern):
//! 1. Project overrides (`codesetu.toml` ignore list + project ignore-config JSON)
//! 2. User global defaults (app data dir)
//! 3. Built-in category patterns
//!
//! Migration never overwrites an existing `codesetu.toml`.

use crate::engine::fsutil::write_atomic;
use crate::engine::paths::codesetu_data_dir;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const IGNORE_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum IgnoreCategory {
    Vcs,
    Deps,
    Build,
    Ide,
    Os,
    Temp,
    Coverage,
    Custom,
}

impl IgnoreCategory {
    pub fn all() -> &'static [IgnoreCategory] {
        &[
            IgnoreCategory::Vcs,
            IgnoreCategory::Deps,
            IgnoreCategory::Build,
            IgnoreCategory::Ide,
            IgnoreCategory::Os,
            IgnoreCategory::Temp,
            IgnoreCategory::Coverage,
            IgnoreCategory::Custom,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            IgnoreCategory::Vcs => "Version control",
            IgnoreCategory::Deps => "Dependencies",
            IgnoreCategory::Build => "Build outputs",
            IgnoreCategory::Ide => "IDE / editor",
            IgnoreCategory::Os => "OS junk",
            IgnoreCategory::Temp => "Temp / caches",
            IgnoreCategory::Coverage => "Coverage / reports",
            IgnoreCategory::Custom => "Custom",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            IgnoreCategory::Vcs => {
                "Skip .git, .svn, .hg metadata so sync never treats VCS as source."
            }
            IgnoreCategory::Deps => "Skip package/vendor trees (node_modules, venv, etc.).",
            IgnoreCategory::Build => "Skip compiled outputs (dist, target, bin/obj).",
            IgnoreCategory::Ide => "Skip editor folders (.idea, .vscode).",
            IgnoreCategory::Os => "Skip OS clutter (.DS_Store, Thumbs.db).",
            IgnoreCategory::Temp => "Skip temporary and cache directories.",
            IgnoreCategory::Coverage => "Skip test coverage artifacts.",
            IgnoreCategory::Custom => "Patterns you add for this machine or project.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnorePattern {
    pub id: String,
    pub pattern: String,
    pub category: IgnoreCategory,
    pub enabled: bool,
    pub builtin: bool,
    #[serde(default)]
    pub source: String, // builtin | user | project | codesetuToml
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreConfigFile {
    pub version: u32,
    pub patterns: Vec<IgnorePattern>,
    /// Category master toggles (disabled category disables all its patterns).
    #[serde(default)]
    pub category_enabled: HashMap<String, bool>,
}

impl Default for IgnoreConfigFile {
    fn default() -> Self {
        Self {
            version: IGNORE_CONFIG_VERSION,
            patterns: builtin_patterns(),
            category_enabled: default_category_flags(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreRulesView {
    pub version: u32,
    pub patterns: Vec<IgnorePattern>,
    pub category_enabled: HashMap<String, bool>,
    pub categories: Vec<CategoryInfo>,
    pub effective_patterns: Vec<String>,
    pub project_path: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub id: String,
    pub label: String,
    pub explanation: String,
    pub enabled: bool,
    pub active_count: u64,
    pub total_count: u64,
}

fn default_category_flags() -> HashMap<String, bool> {
    let mut m = HashMap::new();
    for c in IgnoreCategory::all() {
        m.insert(format!("{:?}", c).to_ascii_lowercase(), true);
        // serde rename is camelCase for keys we use below
    }
    // Use stable camelCase keys
    m.clear();
    m.insert("vcs".into(), true);
    m.insert("deps".into(), true);
    m.insert("build".into(), true);
    m.insert("ide".into(), true);
    m.insert("os".into(), true);
    m.insert("temp".into(), true);
    m.insert("coverage".into(), true);
    m.insert("custom".into(), true);
    m
}

fn category_key(c: IgnoreCategory) -> &'static str {
    match c {
        IgnoreCategory::Vcs => "vcs",
        IgnoreCategory::Deps => "deps",
        IgnoreCategory::Build => "build",
        IgnoreCategory::Ide => "ide",
        IgnoreCategory::Os => "os",
        IgnoreCategory::Temp => "temp",
        IgnoreCategory::Coverage => "coverage",
        IgnoreCategory::Custom => "custom",
    }
}

pub fn builtin_patterns() -> Vec<IgnorePattern> {
    let defs: &[(&str, IgnoreCategory)] = &[
        ("**/.git/**", IgnoreCategory::Vcs),
        ("**/.svn/**", IgnoreCategory::Vcs),
        ("**/.hg/**", IgnoreCategory::Vcs),
        ("**/node_modules/**", IgnoreCategory::Deps),
        ("**/.venv/**", IgnoreCategory::Deps),
        ("**/venv/**", IgnoreCategory::Deps),
        ("**/__pycache__/**", IgnoreCategory::Deps),
        ("**/dist/**", IgnoreCategory::Build),
        ("**/build/**", IgnoreCategory::Build),
        ("**/target/**", IgnoreCategory::Build),
        ("**/bin/**", IgnoreCategory::Build),
        ("**/obj/**", IgnoreCategory::Build),
        ("**/.next/**", IgnoreCategory::Build),
        ("**/.turbo/**", IgnoreCategory::Build),
        ("**/*.exe", IgnoreCategory::Build),
        ("**/*.dll", IgnoreCategory::Build),
        ("**/*.pdb", IgnoreCategory::Build),
        ("**/.idea/**", IgnoreCategory::Ide),
        ("**/.vscode/**", IgnoreCategory::Ide),
        ("**/.DS_Store", IgnoreCategory::Os),
        ("**/Thumbs.db", IgnoreCategory::Os),
        ("**/tmp/**", IgnoreCategory::Temp),
        ("**/.cache/**", IgnoreCategory::Temp),
        ("**/coverage/**", IgnoreCategory::Coverage),
    ];
    defs.iter()
        .enumerate()
        .map(|(i, (pat, cat))| IgnorePattern {
            id: format!("builtin-{i}"),
            pattern: (*pat).to_string(),
            category: *cat,
            enabled: true,
            builtin: true,
            source: "builtin".into(),
        })
        .collect()
}

fn global_config_path() -> PathBuf {
    codesetu_data_dir().join("ignore-rules.json")
}

fn project_config_path(project_root: &Path) -> PathBuf {
    project_root.join(".codesetu").join("ignore-rules.json")
}

pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    let p = pattern.trim();
    if p.is_empty() {
        return Err("Pattern cannot be empty.".into());
    }
    if p.contains('\0') {
        return Err("Pattern contains invalid characters.".into());
    }
    Glob::new(p).map_err(|e| format!("Invalid glob: {e}"))?;
    Ok(())
}

pub fn load_global() -> IgnoreConfigFile {
    let path = global_config_path();
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => {
            let cfg = IgnoreConfigFile::default();
            let _ = save_global(&cfg);
            cfg
        }
    }
}

pub fn save_global(cfg: &IgnoreConfigFile) -> Result<(), String> {
    let path = global_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = cfg.clone();
    out.version = IGNORE_CONFIG_VERSION;
    let raw = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    write_atomic(&path, raw.as_bytes())
}

pub fn load_project(project_root: Option<&Path>) -> Option<IgnoreConfigFile> {
    let root = project_root?;
    let path = project_config_path(root);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_project(project_root: &Path, cfg: &IgnoreConfigFile) -> Result<(), String> {
    let path = project_config_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out = cfg.clone();
    out.version = IGNORE_CONFIG_VERSION;
    let raw = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    write_atomic(&path, raw.as_bytes())
}

/// Merge builtin ← global ← project ← codesetu.toml ignore list.
pub fn effective_config(project_root: Option<&Path>) -> (IgnoreConfigFile, Vec<String>) {
    let mut warnings = Vec::new();
    let mut merged = IgnoreConfigFile::default();
    let global = load_global();
    apply_overlay(&mut merged, &global);

    if let Some(proj) = load_project(project_root) {
        apply_overlay(&mut merged, &proj);
    }

    if let Some(root) = project_root {
        let toml_path = root.join("codesetu.toml");
        if toml_path.exists() {
            match fs::read_to_string(&toml_path) {
                Ok(raw) => match toml::from_str::<TomlIgnoreOnly>(&raw) {
                    Ok(parsed) => {
                        for (i, pat) in parsed.ignore.into_iter().enumerate() {
                            if validate_pattern(&pat).is_ok() {
                                upsert_pattern(
                                    &mut merged,
                                    IgnorePattern {
                                        id: format!("toml-ignore-{i}"),
                                        pattern: pat,
                                        category: IgnoreCategory::Custom,
                                        enabled: true,
                                        builtin: false,
                                        source: "codesetuToml".into(),
                                    },
                                );
                            }
                        }
                    }
                    Err(e) => warnings.push(format!("codesetu.toml parse warning: {e}")),
                },
                Err(e) => warnings.push(format!("Could not read codesetu.toml: {e}")),
            }
        }
    }

    (merged, warnings)
}

#[derive(Debug, Deserialize, Default)]
struct TomlIgnoreOnly {
    #[serde(default)]
    ignore: Vec<String>,
}

fn apply_overlay(base: &mut IgnoreConfigFile, overlay: &IgnoreConfigFile) {
    for (k, v) in &overlay.category_enabled {
        base.category_enabled.insert(k.clone(), *v);
    }
    for p in &overlay.patterns {
        upsert_pattern(base, p.clone());
    }
}

fn upsert_pattern(cfg: &mut IgnoreConfigFile, pat: IgnorePattern) {
    if let Some(existing) = cfg.patterns.iter_mut().find(|p| p.pattern == pat.pattern) {
        // Higher-precedence overlay wins on enabled + identity metadata
        existing.enabled = pat.enabled;
        existing.source = pat.source;
        existing.category = pat.category;
        if !pat.id.is_empty() {
            existing.id = pat.id;
        }
    } else {
        cfg.patterns.push(pat);
    }
}

pub fn build_view(project_root: Option<&Path>) -> IgnoreRulesView {
    let (cfg, warnings) = effective_config(project_root);
    let effective = effective_pattern_strings(&cfg);
    let mut categories = Vec::new();
    for c in IgnoreCategory::all() {
        let key = category_key(*c);
        let enabled = *cfg.category_enabled.get(key).unwrap_or(&true);
        let in_cat: Vec<_> = cfg.patterns.iter().filter(|p| p.category == *c).collect();
        let active = in_cat.iter().filter(|p| p.enabled && enabled).count() as u64;
        categories.push(CategoryInfo {
            id: key.into(),
            label: c.label().into(),
            explanation: c.explanation().into(),
            enabled,
            active_count: active,
            total_count: in_cat.len() as u64,
        });
    }
    IgnoreRulesView {
        version: cfg.version,
        patterns: cfg.patterns,
        category_enabled: cfg.category_enabled,
        categories,
        effective_patterns: effective,
        project_path: project_root.map(|p| p.display().to_string()),
        warnings,
    }
}

pub fn effective_pattern_strings(cfg: &IgnoreConfigFile) -> Vec<String> {
    cfg.patterns
        .iter()
        .filter(|p| {
            let key = category_key(p.category);
            let cat_on = *cfg.category_enabled.get(key).unwrap_or(&true);
            p.enabled && cat_on
        })
        .map(|p| p.pattern.clone())
        .collect()
}

pub fn build_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            builder.add(g);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

pub fn export_config(project_root: Option<&Path>) -> Result<String, String> {
    let (cfg, _) = effective_config(project_root);
    serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())
}

pub fn import_config(
    json: &str,
    project_root: Option<&Path>,
    as_project: bool,
) -> Result<IgnoreRulesView, String> {
    let mut cfg: IgnoreConfigFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
    for p in &cfg.patterns {
        validate_pattern(&p.pattern)?;
    }
    cfg.version = IGNORE_CONFIG_VERSION;
    if as_project {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

pub fn set_pattern_enabled(
    id: &str,
    enabled: bool,
    project_root: Option<&Path>,
    project_scope: bool,
) -> Result<IgnoreRulesView, String> {
    let mut cfg = if project_scope {
        load_project(project_root).unwrap_or_else(|| {
            let (eff, _) = effective_config(project_root);
            eff
        })
    } else {
        load_global()
    };
    let mut found = false;
    for p in &mut cfg.patterns {
        if p.id == id {
            p.enabled = enabled;
            found = true;
            break;
        }
    }
    if !found {
        // Enable/disable by matching against effective and writing override
        let (eff, _) = effective_config(project_root);
        if let Some(p) = eff.patterns.into_iter().find(|p| p.id == id) {
            let mut overlay = p;
            overlay.enabled = enabled;
            overlay.builtin = false;
            overlay.source = if project_scope { "project" } else { "user" }.into();
            upsert_pattern(&mut cfg, overlay);
        } else {
            return Err(format!("Pattern id not found: {id}"));
        }
    }
    if project_scope {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

pub fn set_category_enabled(
    category: &str,
    enabled: bool,
    project_root: Option<&Path>,
    project_scope: bool,
) -> Result<IgnoreRulesView, String> {
    let mut cfg = if project_scope {
        load_project(project_root).unwrap_or_default()
    } else {
        load_global()
    };
    cfg.category_enabled.insert(category.to_string(), enabled);
    if project_scope {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

pub fn add_custom_pattern(
    pattern: &str,
    project_root: Option<&Path>,
    project_scope: bool,
) -> Result<IgnoreRulesView, String> {
    validate_pattern(pattern)?;
    let mut cfg = if project_scope {
        load_project(project_root).unwrap_or_default()
    } else {
        load_global()
    };
    let id = format!("custom-{}", uuid::Uuid::new_v4());
    upsert_pattern(
        &mut cfg,
        IgnorePattern {
            id,
            pattern: pattern.trim().to_string(),
            category: IgnoreCategory::Custom,
            enabled: true,
            builtin: false,
            source: if project_scope { "project" } else { "user" }.into(),
        },
    );
    if project_scope {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

pub fn delete_pattern(
    id: &str,
    project_root: Option<&Path>,
    project_scope: bool,
) -> Result<IgnoreRulesView, String> {
    let mut cfg = if project_scope {
        load_project(project_root).unwrap_or_default()
    } else {
        load_global()
    };
    let before = cfg.patterns.len();
    cfg.patterns.retain(|p| p.id != id || p.builtin);
    // For builtins, disable instead of delete
    for p in &mut cfg.patterns {
        if p.id == id && p.builtin {
            p.enabled = false;
        }
    }
    if cfg.patterns.len() == before {
        // maybe only disable builtin already handled; check id exists in effective
        let (eff, _) = effective_config(project_root);
        if !eff.patterns.iter().any(|p| p.id == id) {
            return Err(format!("Pattern not found: {id}"));
        }
    }
    if project_scope {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

pub fn update_pattern(
    id: &str,
    new_pattern: &str,
    project_root: Option<&Path>,
    project_scope: bool,
) -> Result<IgnoreRulesView, String> {
    validate_pattern(new_pattern)?;
    let mut cfg = if project_scope {
        load_project(project_root).unwrap_or_default()
    } else {
        load_global()
    };
    let mut found = false;
    for p in &mut cfg.patterns {
        if p.id == id {
            if p.builtin {
                return Err(
                    "Built-in patterns cannot be edited; disable and add a custom one.".into(),
                );
            }
            p.pattern = new_pattern.trim().to_string();
            found = true;
            break;
        }
    }
    if !found {
        return Err(format!("Pattern not found: {id}"));
    }
    if project_scope {
        let root = project_root.ok_or_else(|| "No project folder selected.".to_string())?;
        save_project(root, &cfg)?;
    } else {
        save_global(&cfg)?;
    }
    Ok(build_view(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_and_override_precedence() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(dir.path(), || {
            let mut global = IgnoreConfigFile::default();
            for p in &mut global.patterns {
                if p.pattern.contains("node_modules") {
                    p.enabled = false;
                }
            }
            save_global(&global).unwrap();
            let (cfg, _) = effective_config(None);
            let eff = effective_pattern_strings(&cfg);
            assert!(!eff.iter().any(|p| p.contains("node_modules")));
            assert!(eff.iter().any(|p| p.contains(".git")));
        });
    }

    #[test]
    fn does_not_overwrite_codesetu_toml() {
        let dir = tempdir().unwrap();
        crate::engine::paths::with_test_data_dir(dir.path(), || {
            let project = dir.path().join("proj");
            fs::create_dir_all(&project).unwrap();
            let toml = project.join("codesetu.toml");
            fs::write(&toml, "ignore = [\"**/mine/**\"]\n").unwrap();
            let before = fs::read_to_string(&toml).unwrap();
            let _ = build_view(Some(&project));
            let after = fs::read_to_string(&toml).unwrap();
            assert_eq!(before, after);
            let view = build_view(Some(&project));
            assert!(view.effective_patterns.iter().any(|p| p.contains("mine")));
        });
    }

    #[test]
    fn validate_rejects_bad_glob() {
        assert!(validate_pattern("").is_err());
        assert!(validate_pattern("**/ok/**").is_ok());
    }
}
