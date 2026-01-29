use crate::categories::{CategoryPatterns, FileCategory};
use crate::config::Config;
use bytesize::ByteSize;
use chrono::{DateTime, Duration, Utc};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScannedItem {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub category: FileCategory,
    pub confidence: f32,
    pub is_dir: bool,
    pub modified: DateTime<Utc>,
    pub accessed: DateTime<Utc>,
    pub is_stale: bool,
    pub hash: Option<String>,
}

impl ScannedItem {
    pub fn size_formatted(&self) -> String {
        ByteSize(self.size).to_string()
    }

    pub fn confidence_percent(&self) -> u8 {
        (self.confidence * 100.0) as u8
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanStats {
    pub total_items: u64,
    pub total_size: u64,
    pub items_by_category: HashMap<FileCategory, u64>,
    pub size_by_category: HashMap<FileCategory, u64>,
    pub duration_ms: u64,
}

impl ScanStats {
    pub fn total_size_formatted(&self) -> String {
        ByteSize(self.total_size).to_string()
    }
}

pub struct Scanner {
    config: Config,
    items: Arc<Mutex<Vec<ScannedItem>>>,
    stats: Arc<Mutex<ScanStats>>,
    is_scanning: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    files_scanned: Arc<AtomicU64>,
    current_path: Arc<Mutex<String>>,
}

impl Scanner {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            items: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(ScanStats::default())),
            is_scanning: Arc::new(AtomicBool::new(false)),
            should_stop: Arc::new(AtomicBool::new(false)),
            files_scanned: Arc::new(AtomicU64::new(0)),
            current_path: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn is_scanning(&self) -> bool {
        self.is_scanning.load(Ordering::SeqCst)
    }

    pub fn files_scanned(&self) -> u64 {
        self.files_scanned.load(Ordering::SeqCst)
    }

    pub fn current_path(&self) -> String {
        self.current_path
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn scan(&self) -> Vec<ScannedItem> {
        self.is_scanning.store(true, Ordering::SeqCst);
        self.should_stop.store(false, Ordering::SeqCst);
        self.files_scanned.store(0, Ordering::SeqCst);

        if let Ok(mut items) = self.items.lock() {
            items.clear();
        }
        if let Ok(mut stats) = self.stats.lock() {
            *stats = ScanStats::default();
        }

        let start_time = std::time::Instant::now();

        rayon::scope(|s| {
            s.spawn(|_| self.scan_known_cache_paths());
            s.spawn(|_| self.scan_project_directories());
            s.spawn(|_| self.scan_downloads());
        });

        let duration = start_time.elapsed();
        if let Ok(mut stats) = self.stats.lock() {
            stats.duration_ms = duration.as_millis() as u64;
        }

        self.is_scanning.store(false, Ordering::SeqCst);

        self.items
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn scan_known_cache_paths(&self) {
        if self.should_stop.load(Ordering::SeqCst) {
            return;
        }

        let cache_configs: Vec<(Vec<String>, FileCategory)> = vec![
            (
                CategoryPatterns::package_cache_paths(),
                FileCategory::PackageCache,
            ),
            (CategoryPatterns::ide_cache_paths(), FileCategory::IdeCache),
            (
                CategoryPatterns::browser_cache_paths(),
                FileCategory::BrowserCache,
            ),
        ];

        cache_configs.into_par_iter().for_each(|(paths, category)| {
            if self.should_stop.load(Ordering::SeqCst) {
                return;
            }

            paths.par_iter().for_each(|cache_path| {
                if self.should_stop.load(Ordering::SeqCst) {
                    return;
                }

                let path = PathBuf::from(cache_path);
                if path.exists() && path.is_dir() {
                    self.update_current_path(&path);

                    if let Ok(size) = Self::dir_size_parallel(&path) {
                        if size > 0 {
                            let meta = fs::metadata(&path).ok();
                            let modified = meta
                                .as_ref()
                                .and_then(|m| m.modified().ok())
                                .map(DateTime::<Utc>::from)
                                .unwrap_or_else(Utc::now);

                            let item = ScannedItem {
                                path: path.clone(),
                                name: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                size,
                                category,
                                confidence: category.base_confidence(),
                                is_dir: true,
                                modified,
                                accessed: modified,
                                is_stale: false,
                                hash: None,
                            };

                            self.add_item(item);
                        }
                    }
                }
            });
        });

        self.scan_system_caches();
    }

    fn scan_system_caches(&self) {
        if self.should_stop.load(Ordering::SeqCst) {
            return;
        }

        let system_paths = CategoryPatterns::system_cache_paths();

        system_paths.par_iter().for_each(|cache_path| {
            if self.should_stop.load(Ordering::SeqCst) {
                return;
            }

            let path = PathBuf::from(cache_path);
            if !path.exists() || !path.is_dir() {
                return;
            }

            self.update_current_path(&path);

            let entries: Vec<_> = WalkDir::new(&path)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.depth() == 1 && e.file_type().is_dir())
                .collect();

            entries.par_iter().for_each(|entry| {
                if self.should_stop.load(Ordering::SeqCst) {
                    return;
                }

                if let Ok(size) = Self::dir_size_parallel(entry.path()) {
                    if size > 1024 * 1024 {
                        let meta = entry.metadata().ok();
                        let modified = meta
                            .as_ref()
                            .and_then(|m| m.modified().ok())
                            .map(DateTime::<Utc>::from)
                            .unwrap_or_else(Utc::now);

                        let item = ScannedItem {
                            path: entry.path().to_path_buf(),
                            name: entry.file_name().to_string_lossy().to_string(),
                            size,
                            category: FileCategory::SystemCache,
                            confidence: FileCategory::SystemCache.base_confidence(),
                            is_dir: true,
                            modified,
                            accessed: modified,
                            is_stale: false,
                            hash: None,
                        };

                        self.add_item(item);
                    }
                }
            });
        });
    }

    fn scan_project_directories(&self) {
        if self.should_stop.load(Ordering::SeqCst) {
            return;
        }

        let dev_dirs: HashSet<&str> = CategoryPatterns::dev_artifact_dirs()
            .iter()
            .copied()
            .collect();
        let temp_exts = CategoryPatterns::temp_extensions();
        let log_exts = CategoryPatterns::log_extensions();
        let found_artifacts: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

        for base_path in &self.config.scan_paths {
            if !base_path.exists() || self.should_stop.load(Ordering::SeqCst) {
                continue;
            }

            self.update_current_path(base_path);

            let walker = WalkDir::new(base_path)
                .max_depth(self.config.max_depth as usize)
                .follow_links(self.config.follow_symlinks)
                .into_iter();

            let mut pending_artifacts: Vec<(PathBuf, String, std::fs::Metadata)> = Vec::new();

            for entry in walker.filter_entry(|e| {
                let dominated = found_artifacts
                    .lock()
                    .map(|guard| guard.iter().any(|artifact| e.path().starts_with(artifact)))
                    .unwrap_or(false);
                !dominated
            }) {
                if self.should_stop.load(Ordering::SeqCst) {
                    break;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                self.files_scanned.fetch_add(1, Ordering::SeqCst);

                if entry.file_type().is_dir() {
                    let name = entry.file_name().to_string_lossy();
                    if dev_dirs.contains(name.as_ref()) {
                        if let Ok(mut guard) = found_artifacts.lock() {
                            guard.insert(entry.path().to_path_buf());
                        }

                        if let Ok(meta) = entry.metadata() {
                            pending_artifacts.push((
                                entry.path().to_path_buf(),
                                name.to_string(),
                                meta,
                            ));
                        }
                    }
                } else if entry.file_type().is_file() {
                    let name = entry.file_name().to_string_lossy();
                    let name_lower = name.to_lowercase();

                    let is_temp = temp_exts.iter().any(|ext| name_lower.ends_with(ext));
                    let is_log = log_exts.iter().any(|ext| name_lower.ends_with(ext));

                    if is_temp || is_log {
                        if let Ok(meta) = entry.metadata() {
                            let size = meta.len();
                            if size > 0 {
                                let modified = meta
                                    .modified()
                                    .ok()
                                    .map(DateTime::<Utc>::from)
                                    .unwrap_or_else(Utc::now);

                                let category = if is_temp {
                                    FileCategory::TempFile
                                } else {
                                    FileCategory::LogFile
                                };

                                let item = ScannedItem {
                                    path: entry.path().to_path_buf(),
                                    name: name.to_string(),
                                    size,
                                    category,
                                    confidence: category.base_confidence(),
                                    is_dir: false,
                                    modified,
                                    accessed: modified,
                                    is_stale: false,
                                    hash: None,
                                };

                                self.add_item(item);
                            }
                        }
                    }
                }
            }

            pending_artifacts.par_iter().for_each(|(path, name, meta)| {
                if self.should_stop.load(Ordering::SeqCst) {
                    return;
                }

                if let Ok(size) = Self::dir_size_parallel(path) {
                    let modified = meta
                        .modified()
                        .ok()
                        .map(DateTime::<Utc>::from)
                        .unwrap_or_else(Utc::now);

                    let age_days = (Utc::now() - modified).num_days() as u64;
                    let is_stale = age_days >= CategoryPatterns::stale_threshold_days();

                    let mut confidence = FileCategory::DevArtifact.base_confidence();
                    if is_stale {
                        confidence += 0.10;
                    }
                    confidence = confidence.min(0.98);

                    let item = ScannedItem {
                        path: path.clone(),
                        name: name.clone(),
                        size,
                        category: FileCategory::DevArtifact,
                        confidence,
                        is_dir: true,
                        modified,
                        accessed: modified,
                        is_stale,
                        hash: None,
                    };

                    self.add_item(item);
                }
            });
        }
    }

    fn scan_downloads(&self) {
        if self.should_stop.load(Ordering::SeqCst) {
            return;
        }

        let downloads = match dirs::download_dir() {
            Some(d) if d.exists() => d,
            _ => return,
        };

        self.update_current_path(&downloads);

        let threshold = Utc::now() - Duration::days(CategoryPatterns::old_download_days() as i64);

        let entries: Vec<_> = WalkDir::new(&downloads)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.depth() == 1 && e.file_type().is_file())
            .collect();

        entries.par_iter().for_each(|entry| {
            if self.should_stop.load(Ordering::SeqCst) {
                return;
            }

            if let Ok(meta) = entry.metadata() {
                let modified = meta
                    .modified()
                    .ok()
                    .map(DateTime::<Utc>::from)
                    .unwrap_or_else(Utc::now);

                if modified < threshold {
                    let size = meta.len();
                    let age_days = (Utc::now() - modified).num_days() as u64;
                    let is_stale = age_days >= CategoryPatterns::stale_threshold_days();

                    let mut confidence = FileCategory::OldDownload.base_confidence();
                    if is_stale {
                        confidence += 0.10;
                    }
                    confidence = confidence.min(0.95);

                    let item = ScannedItem {
                        path: entry.path().to_path_buf(),
                        name: entry.file_name().to_string_lossy().to_string(),
                        size,
                        category: FileCategory::OldDownload,
                        confidence,
                        is_dir: false,
                        modified,
                        accessed: modified,
                        is_stale,
                        hash: None,
                    };

                    self.add_item(item);
                }
            }
        });
    }

    fn add_item(&self, item: ScannedItem) {
        let Ok(mut items) = self.items.lock() else {
            return;
        };
        let Ok(mut stats) = self.stats.lock() else {
            items.push(item);
            return;
        };

        stats.total_items += 1;
        stats.total_size += item.size;
        *stats.items_by_category.entry(item.category).or_insert(0) += 1;
        *stats.size_by_category.entry(item.category).or_insert(0) += item.size;

        items.push(item);
    }

    fn update_current_path(&self, path: &Path) {
        if let Ok(mut current) = self.current_path.lock() {
            *current = path.to_string_lossy().to_string();
        }
    }

    fn dir_size_parallel(path: &Path) -> Result<u64, std::io::Error> {
        let entries: Vec<_> = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        let size: u64 = entries
            .par_iter()
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .sum();

        Ok(size)
    }

    pub fn get_stats(&self) -> ScanStats {
        self.stats
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn get_items(&self) -> Vec<ScannedItem> {
        self.items
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}
