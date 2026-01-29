use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileCategory {
    DevArtifact,
    PackageCache,
    IdeCache,
    BrowserCache,
    SystemCache,
    LogFile,
    TempFile,
    LargeFile,
    OldDownload,
    Duplicate,
    Unused,
}

impl FileCategory {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DevArtifact => "Dev Artifacts",
            Self::PackageCache => "Package Caches",
            Self::IdeCache => "IDE Caches",
            Self::BrowserCache => "Browser Caches",
            Self::SystemCache => "System Caches",
            Self::LogFile => "Log Files",
            Self::TempFile => "Temp Files",
            Self::LargeFile => "Large Files",
            Self::OldDownload => "Old Downloads",
            Self::Duplicate => "Duplicates",
            Self::Unused => "Unused Files",
        }
    }

    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::DevArtifact => "code",
            Self::PackageCache => "package",
            Self::IdeCache => "cpu",
            Self::BrowserCache => "globe",
            Self::SystemCache => "hard-drive",
            Self::LogFile => "file-text",
            Self::TempFile => "file-x",
            Self::LargeFile => "file-archive",
            Self::OldDownload => "download",
            Self::Duplicate => "copy",
            Self::Unused => "clock",
        }
    }

    pub fn base_confidence(&self) -> f32 {
        match self {
            Self::TempFile => 0.95,
            Self::SystemCache => 0.92,
            Self::BrowserCache => 0.90,
            Self::PackageCache => 0.88,
            Self::IdeCache => 0.85,
            Self::LogFile => 0.85,
            Self::DevArtifact => 0.80,
            Self::OldDownload => 0.75,
            Self::LargeFile => 0.70,
            Self::Duplicate => 0.70,
            Self::Unused => 0.70,
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Self::DevArtifact => "Build outputs and dependencies (node_modules, target/, build/)",
            Self::PackageCache => "Package manager caches (npm, cargo, pip, etc.)",
            Self::IdeCache => "IDE and editor caches (VS Code, JetBrains, etc.)",
            Self::BrowserCache => "Web browser cached data",
            Self::SystemCache => "System and application caches",
            Self::LogFile => "Application and system log files",
            Self::TempFile => "Temporary files and directories",
            Self::LargeFile => "Files larger than 100MB",
            Self::OldDownload => "Downloaded files older than 30 days",
            Self::Duplicate => "Files with identical content",
            Self::Unused => "Files not accessed in 90+ days",
        }
    }
}

pub struct CategoryPatterns;

impl CategoryPatterns {
    pub fn dev_artifact_dirs() -> &'static [&'static str] {
        &[
            "node_modules",
            "target",
            "build",
            "dist",
            ".build",
            "Pods",
            "DerivedData",
            ".gradle",
            "out",
            "bin",
            "obj",
            "__pycache__",
            ".pytest_cache",
            ".mypy_cache",
            "venv",
            ".venv",
            "vendor",
            ".next",
            ".nuxt",
            ".output",
            "coverage",
            ".coverage",
            ".tox",
            "eggs",
            "*.egg-info",
            ".eggs",
            "bower_components",
        ]
    }

    pub fn package_cache_paths() -> Vec<String> {
        let mut paths = Vec::new();
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return paths,
        };
        let home_str = home.to_string_lossy();

        paths.extend([
            format!("{}/.npm", home_str),
            format!("{}/.yarn/cache", home_str),
            format!("{}/.pnpm-store", home_str),
            format!("{}/.cargo/registry/cache", home_str),
            format!("{}/.cargo/git/db", home_str),
            format!("{}/.gradle/caches", home_str),
            format!("{}/.m2/repository", home_str),
            format!("{}/.gem/cache", home_str),
            format!("{}/.composer/cache", home_str),
            format!("{}/.nuget/packages", home_str),
        ]);

        #[cfg(target_os = "macos")]
        paths.extend([
            format!("{}/Library/Caches/Homebrew", home_str),
            format!("{}/Library/Caches/CocoaPods", home_str),
        ]);

        #[cfg(target_os = "linux")]
        paths.extend([
            format!("{}/.cache/pip", home_str),
            format!("{}/.cache/go-build", home_str),
        ]);

        #[cfg(target_os = "windows")]
        if let Some(local_app) = dirs::data_local_dir() {
            let local_str = local_app.to_string_lossy();
            paths.extend([
                format!("{}/pip/Cache", local_str),
                format!("{}/go-build", local_str),
                format!("{}/NuGet/packages", local_str),
            ]);
        }

        paths
    }

    pub fn ide_cache_paths() -> Vec<String> {
        let mut paths = Vec::new();
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return paths,
        };
        let home_str = home.to_string_lossy();

        paths.extend([
            format!("{}/.vscode/extensions", home_str),
            format!("{}/.cursor/extensions", home_str),
            format!("{}/.zed/extensions", home_str),
        ]);

        #[cfg(target_os = "macos")]
        {
            paths.extend([
                format!(
                    "{}/Library/Application Support/Code/CachedExtensions",
                    home_str
                ),
                format!("{}/Library/Application Support/Code/CachedData", home_str),
                format!("{}/Library/Caches/com.microsoft.VSCode", home_str),
                format!("{}/Library/Caches/JetBrains", home_str),
            ]);
        }

        #[cfg(target_os = "linux")]
        {
            paths.extend([
                format!("{}/.local/share/JetBrains", home_str),
                format!("{}/.config/Code/CachedData", home_str),
                format!("{}/.config/Code/CachedExtensions", home_str),
            ]);
        }

        #[cfg(target_os = "windows")]
        if let Some(app_data) = dirs::config_dir() {
            let app_str = app_data.to_string_lossy();
            paths.extend([
                format!("{}/Code/CachedData", app_str),
                format!("{}/Code/CachedExtensions", app_str),
                format!("{}/JetBrains", app_str),
            ]);
        }

        paths
    }

    pub fn browser_cache_paths() -> Vec<String> {
        let mut paths = Vec::new();
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return paths,
        };
        let home_str = home.to_string_lossy();

        #[cfg(target_os = "macos")]
        {
            paths.extend([
                format!("{}/Library/Caches/Google/Chrome", home_str),
                format!("{}/Library/Caches/com.apple.Safari", home_str),
                format!("{}/Library/Caches/Firefox", home_str),
                format!("{}/Library/Caches/Microsoft Edge", home_str),
                format!("{}/Library/Caches/BraveSoftware", home_str),
            ]);
        }

        #[cfg(target_os = "linux")]
        {
            paths.extend([
                format!("{}/.cache/google-chrome", home_str),
                format!("{}/.cache/mozilla/firefox", home_str),
                format!("{}/.cache/chromium", home_str),
                format!("{}/.cache/microsoft-edge", home_str),
                format!("{}/.cache/BraveSoftware", home_str),
            ]);
        }

        #[cfg(target_os = "windows")]
        if let Some(local_app) = dirs::data_local_dir() {
            let local_str = local_app.to_string_lossy();
            paths.extend([
                format!("{}/Google/Chrome/User Data/Default/Cache", local_str),
                format!("{}/Mozilla/Firefox/Profiles", local_str),
                format!("{}/Microsoft/Edge/User Data/Default/Cache", local_str),
                format!(
                    "{}/BraveSoftware/Brave-Browser/User Data/Default/Cache",
                    local_str
                ),
            ]);
        }

        paths
    }

    pub fn system_cache_paths() -> Vec<String> {
        let mut paths = Vec::new();
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return paths,
        };
        let home_str = home.to_string_lossy();

        #[cfg(target_os = "macos")]
        {
            paths.push(format!("{}/Library/Caches", home_str));
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            paths.push(format!("{}/.cache", home_str));
            if std::path::Path::new("/tmp").exists() {
                paths.push("/tmp".to_string());
            }
            if std::path::Path::new("/var/tmp").exists() {
                paths.push("/var/tmp".to_string());
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(local_app) = dirs::data_local_dir() {
                paths.push(format!("{}/Temp", local_app.to_string_lossy()));
            }
            if let Some(temp) = std::env::var_os("TEMP") {
                paths.push(temp.to_string_lossy().to_string());
            }
        }

        paths
    }

    pub fn temp_extensions() -> &'static [&'static str] {
        &[
            ".tmp",
            ".temp",
            ".bak",
            ".swp",
            ".swo",
            "~",
            ".old",
            ".orig",
            ".DS_Store",
            "Thumbs.db",
            "._.DS_Store",
            "desktop.ini",
            ".crdownload",
            ".part",
            ".partial",
        ]
    }

    pub fn log_extensions() -> &'static [&'static str] {
        &[".log", ".logs"]
    }

    pub fn stale_threshold_days() -> u64 {
        90
    }

    pub fn old_download_days() -> u64 {
        30
    }

    #[allow(dead_code)]
    pub fn large_file_threshold() -> u64 {
        100 * 1024 * 1024 // 100MB
    }
}
