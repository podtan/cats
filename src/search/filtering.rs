use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SearchFilteringConfig {
    pub enabled: Option<bool>,
    pub exclude_dirs: Option<Vec<String>>,
    pub exclude_extensions: Option<Vec<String>>,
    pub exclude_hidden: Option<bool>,
}

impl Default for SearchFilteringConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            exclude_dirs: Some(vec![
                "target".to_string(),
                "node_modules".to_string(),
                "__pycache__".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".git".to_string(),
                ".svn".to_string(),
                ".hg".to_string(),
                "venv".to_string(),
                "env".to_string(),
                ".venv".to_string(),
            ]),
            exclude_extensions: Some(vec![
                "exe".to_string(),
                "dll".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "a".to_string(),
                "o".to_string(),
                "pyc".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "bmp".to_string(),
                "ico".to_string(),
                "mp3".to_string(),
                "mp4".to_string(),
                "avi".to_string(),
                "mov".to_string(),
                "wav".to_string(),
                "pdf".to_string(),
                "zip".to_string(),
                "tar".to_string(),
                "gz".to_string(),
                "rar".to_string(),
                "7z".to_string(),
            ]),
            exclude_hidden: Some(true),
        }
    }
}

pub struct ConfigurableFilter {
    pub config: SearchFilteringConfig,
}

impl ConfigurableFilter {
    /// Create a new filter. If `config` is `None`, attempt to read
    /// `config/simpaticoder.toml` and use its `[search_filtering]` section
    /// if present. Otherwise fall back to defaults.
    pub fn new(config: Option<SearchFilteringConfig>) -> Self {
        if let Some(cfg) = config {
            return Self { config: cfg };
        }

        // Try to read local config file
        let mut final_cfg = SearchFilteringConfig::default();
        if let Ok(content) = fs::read_to_string("config/simpaticoder.toml") {
            #[derive(serde::Deserialize)]
            struct PartialConfig {
                search_filtering: Option<SearchFilteringConfig>,
            }

            if let Ok(partial) = toml::from_str::<PartialConfig>(&content) {
                if let Some(sf) = partial.search_filtering {
                    final_cfg = sf;
                }
            }
        }

        Self { config: final_cfg }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled.unwrap_or(true)
    }

    pub fn should_include_path(&self, path: &Path) -> bool {
        if !self.is_enabled() {
            return true;
        }

        // Hidden files/directories
        if self.config.exclude_hidden.unwrap_or(true) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    return false;
                }
            }
        }

        // Exclude by directory name (if any component matches)
        if let Some(ex_dirs) = &self.config.exclude_dirs {
            for comp in path.iter() {
                if let Some(s) = comp.to_str() {
                    if ex_dirs.iter().any(|d| d == s) {
                        return false;
                    }
                }
            }
        }

        // Exclude by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(ex_exts) = &self.config.exclude_extensions {
                let ext_l = ext.to_lowercase();
                if ex_exts.iter().any(|e| e == &ext_l) {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_excludes_common_dirs() {
        let filter = ConfigurableFilter::new(Some(SearchFilteringConfig::default()));
        assert!(!filter.should_include_path(Path::new("target")));
        assert!(!filter.should_include_path(Path::new("node_modules")));
        assert!(filter.should_include_path(Path::new("src")));
        // Component match: containing a protected component should be excluded
        assert!(!filter.should_include_path(Path::new("some/.git/config")));
    }

    #[test]
    fn disabled_filter_allows_everything() {
        let cfg = SearchFilteringConfig {
            enabled: Some(false),
            exclude_dirs: None,
            exclude_extensions: None,
            exclude_hidden: None,
        };
        let filter = ConfigurableFilter::new(Some(cfg));
        assert!(filter.should_include_path(Path::new("target")));
        assert!(filter.should_include_path(Path::new(".git")));
        assert!(filter.should_include_path(Path::new("binary.exe")));
    }

    #[test]
    fn extension_exclusion_works() {
        let cfg = SearchFilteringConfig {
            enabled: Some(true),
            exclude_dirs: None,
            exclude_extensions: Some(vec!["exe".to_string()]),
            exclude_hidden: Some(true),
        };
        let filter = ConfigurableFilter::new(Some(cfg));
        assert!(!filter.should_include_path(Path::new("run.exe")));
        assert!(filter.should_include_path(Path::new("run.rs")));
    }

    #[test]
    fn hidden_files_excluded_when_enabled() {
        let cfg = SearchFilteringConfig {
            enabled: Some(true),
            exclude_dirs: None,
            exclude_extensions: None,
            exclude_hidden: Some(true),
        };
        let filter = ConfigurableFilter::new(Some(cfg));
        assert!(!filter.should_include_path(Path::new(".hiddenfile")));
        assert!(!filter.should_include_path(Path::new("dir/.hidden")));
        assert!(filter.should_include_path(Path::new("visible")));
    }
}
