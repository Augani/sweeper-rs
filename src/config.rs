use crate::categories::FileCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub scan_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub enabled_categories: HashSet<FileCategory>,
    pub large_file_threshold: u64,
    pub stale_days_threshold: u64,
    pub old_download_days: u64,
    pub show_hidden: bool,
    pub follow_symlinks: bool,
    pub max_depth: u32,
    pub use_trash: bool,
    pub dry_run: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut enabled_categories = HashSet::new();
        enabled_categories.insert(FileCategory::DevArtifact);
        enabled_categories.insert(FileCategory::PackageCache);
        enabled_categories.insert(FileCategory::IdeCache);
        enabled_categories.insert(FileCategory::BrowserCache);
        enabled_categories.insert(FileCategory::SystemCache);
        enabled_categories.insert(FileCategory::LogFile);
        enabled_categories.insert(FileCategory::TempFile);
        enabled_categories.insert(FileCategory::LargeFile);
        enabled_categories.insert(FileCategory::OldDownload);

        Self {
            scan_paths: Self::default_scan_paths(),
            exclude_patterns: Self::default_excludes(),
            enabled_categories,
            large_file_threshold: 100 * 1024 * 1024,
            stale_days_threshold: 90,
            old_download_days: 30,
            show_hidden: true,
            follow_symlinks: false,
            max_depth: 20,
            use_trash: true,
            dry_run: false,
        }
    }
}

impl Config {
    pub fn default_scan_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Some(home) = dirs::home_dir() {
            let common_dev_dirs = ["Projects", "Developer", "Code", "dev", "repos", "workspace"];
            for dir in common_dev_dirs {
                let path = home.join(dir);
                if path.exists() {
                    paths.push(path);
                }
            }

            if let Some(downloads) = dirs::download_dir() {
                if downloads.exists() {
                    paths.push(downloads);
                }
            }

            #[cfg(target_os = "macos")]
            {
                let caches = home.join("Library/Caches");
                if caches.exists() {
                    paths.push(caches);
                }
            }

            #[cfg(target_os = "linux")]
            {
                let cache = home.join(".cache");
                if cache.exists() {
                    paths.push(cache);
                }
            }

            #[cfg(target_os = "windows")]
            {
                if let Some(local_app) = dirs::data_local_dir() {
                    let temp = local_app.join("Temp");
                    if temp.exists() {
                        paths.push(temp);
                    }
                }
            }
        }

        if paths.is_empty() {
            if let Some(home) = dirs::home_dir() {
                paths.push(home);
            }
        }

        paths
    }

    pub fn default_excludes() -> Vec<String> {
        let mut excludes = vec![".git".to_string(), ".svn".to_string(), ".hg".to_string()];

        #[cfg(target_os = "macos")]
        excludes.extend([
            "System".to_string(),
            "Applications".to_string(),
            "/usr".to_string(),
            "/bin".to_string(),
            "/sbin".to_string(),
            "/Library/Apple".to_string(),
        ]);

        #[cfg(target_os = "linux")]
        excludes.extend([
            "/usr".to_string(),
            "/bin".to_string(),
            "/sbin".to_string(),
            "/etc".to_string(),
            "/var/lib".to_string(),
        ]);

        #[cfg(target_os = "windows")]
        excludes.extend([
            "Windows".to_string(),
            "Program Files".to_string(),
            "Program Files (x86)".to_string(),
            "$Recycle.Bin".to_string(),
            "System Volume Information".to_string(),
        ]);

        excludes
    }

    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, contents)
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sweeper")
            .join("config.json")
    }
}
