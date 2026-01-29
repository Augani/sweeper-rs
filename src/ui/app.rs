use crate::categories::FileCategory;
use crate::config::Config;
use crate::scanner::{ScanStats, ScannedItem, Scanner};
use adabraka_ui::components::button::{Button, ButtonSize, ButtonVariant};
use adabraka_ui::components::checkbox::Checkbox;
use adabraka_ui::components::icon::Icon;
use adabraka_ui::components::scrollable::scrollable_vertical;
use adabraka_ui::components::sparkline::Sparkline;
use adabraka_ui::components::spinner::Spinner;
use adabraka_ui::display::badge::{Badge, BadgeVariant};
use adabraka_ui::display::card::Card;
use adabraka_ui::prelude::*;
use gpui::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterTab {
    All,
    Largest,
    DevArtifacts,
    Caches,
    TempLogs,
    Downloads,
}

impl FilterTab {
    fn label(&self) -> &'static str {
        match self {
            Self::All => "All Files",
            Self::Largest => "Largest",
            Self::DevArtifacts => "Dev Artifacts",
            Self::Caches => "Caches",
            Self::TempLogs => "Temp & Logs",
            Self::Downloads => "Downloads",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Self::All => "layers",
            Self::Largest => "arrow-down-wide-narrow",
            Self::DevArtifacts => "code",
            Self::Caches => "database",
            Self::TempLogs => "file-text",
            Self::Downloads => "download",
        }
    }
}

fn category_icon(category: FileCategory) -> &'static str {
    match category {
        FileCategory::DevArtifact => "code",
        FileCategory::PackageCache => "package",
        FileCategory::IdeCache => "braces",
        FileCategory::BrowserCache => "globe",
        FileCategory::SystemCache => "server",
        FileCategory::TempFile => "file-x",
        FileCategory::LogFile => "file-text",
        FileCategory::OldDownload => "download",
        FileCategory::LargeFile => "file-archive",
        FileCategory::Duplicate => "copy",
        FileCategory::Unused => "clock",
    }
}

pub struct SweeperApp {
    config: Config,
    scanner: Arc<Scanner>,
    items: Vec<ScannedItem>,
    selected: HashSet<PathBuf>,
    stats: ScanStats,
    active_tab: FilterTab,
    is_scanning: bool,
    scan_progress: String,
    show_delete_dialog: bool,
    is_deleting: bool,
}

impl SweeperApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = Config::load();
        let scanner = Arc::new(Scanner::new(config.clone()));

        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;

            let should_continue = this
                .update(cx, |app, cx| {
                    if app.scanner.is_scanning() {
                        app.is_scanning = true;
                        let files_scanned = app.scanner.files_scanned();
                        app.scan_progress = format!(
                            "{} paths checked • {}",
                            files_scanned,
                            app.scanner.current_path()
                        );
                        cx.notify();
                    } else if app.is_scanning {
                        app.is_scanning = false;
                        app.items = app.scanner.get_items();
                        app.stats = app.scanner.get_stats();
                        app.scan_progress = String::new();
                        cx.notify();
                    }
                })
                .is_ok();

            if !should_continue {
                break;
            }
        })
        .detach();

        Self {
            config,
            scanner,
            items: Vec::new(),
            selected: HashSet::new(),
            stats: ScanStats::default(),
            active_tab: FilterTab::All,
            is_scanning: false,
            scan_progress: String::new(),
            show_delete_dialog: false,
            is_deleting: false,
        }
    }

    fn start_scan(&mut self, cx: &mut Context<Self>) {
        self.is_scanning = true;
        self.items.clear();
        self.selected.clear();
        self.scan_progress = "Starting scan...".to_string();
        cx.notify();

        let scanner = self.scanner.clone();
        cx.spawn(async move |this, cx| {
            let (items, stats) = cx
                .background_executor()
                .spawn(async move {
                    let items = scanner.scan();
                    let stats = scanner.get_stats();
                    (items, stats)
                })
                .await;

            let _ = this.update(cx, |app, cx| {
                app.items = items;
                app.stats = stats;
                app.is_scanning = false;
                app.scan_progress = String::new();
                cx.notify();
            });
        })
        .detach();
    }

    fn filtered_items(&self) -> Vec<&ScannedItem> {
        let mut items: Vec<&ScannedItem> = match self.active_tab {
            FilterTab::All => self.items.iter().collect(),
            FilterTab::Largest => {
                let mut sorted: Vec<_> = self.items.iter().collect();
                sorted.sort_by(|a, b| b.size.cmp(&a.size));
                sorted
            }
            FilterTab::DevArtifacts => self
                .items
                .iter()
                .filter(|i| i.category == FileCategory::DevArtifact)
                .collect(),
            FilterTab::Caches => self
                .items
                .iter()
                .filter(|i| {
                    matches!(
                        i.category,
                        FileCategory::PackageCache
                            | FileCategory::IdeCache
                            | FileCategory::BrowserCache
                            | FileCategory::SystemCache
                    )
                })
                .collect(),
            FilterTab::TempLogs => self
                .items
                .iter()
                .filter(|i| matches!(i.category, FileCategory::TempFile | FileCategory::LogFile))
                .collect(),
            FilterTab::Downloads => self
                .items
                .iter()
                .filter(|i| i.category == FileCategory::OldDownload)
                .collect(),
        };

        if self.active_tab != FilterTab::Largest {
            items.sort_by(|a, b| b.size.cmp(&a.size));
        }

        items
    }

    fn toggle_selection(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.selected.contains(&path) {
            self.selected.remove(&path);
        } else {
            self.selected.insert(path);
        }
        cx.notify();
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<PathBuf> = self
            .filtered_items()
            .iter()
            .map(|i| i.path.clone())
            .collect();
        for path in paths {
            self.selected.insert(path);
        }
        cx.notify();
    }

    fn deselect_all(&mut self, cx: &mut Context<Self>) {
        self.selected.clear();
        cx.notify();
    }

    fn selected_size(&self) -> u64 {
        self.items
            .iter()
            .filter(|i| self.selected.contains(&i.path))
            .map(|i| i.size)
            .sum()
    }

    fn delete_selected(&mut self, cx: &mut Context<Self>) {
        self.is_deleting = true;
        self.show_delete_dialog = false;
        cx.notify();

        let paths_to_delete: Vec<PathBuf> = self
            .items
            .iter()
            .filter(|i| self.selected.contains(&i.path))
            .map(|i| i.path.clone())
            .collect();

        let use_trash = self.config.use_trash;
        let dry_run = self.config.dry_run;

        cx.spawn(async move |this, cx| {
            let deleted_paths: HashSet<PathBuf> = cx
                .background_executor()
                .spawn(async move {
                    let mut deleted = HashSet::new();
                    for path in paths_to_delete {
                        let success = if use_trash {
                            trash::delete(&path).is_ok()
                        } else if !dry_run {
                            if path.is_dir() {
                                std::fs::remove_dir_all(&path).is_ok()
                            } else {
                                std::fs::remove_file(&path).is_ok()
                            }
                        } else {
                            true
                        };
                        if success {
                            deleted.insert(path);
                        }
                    }
                    deleted
                })
                .await;

            let _ = this.update(cx, |app, cx| {
                app.items.retain(|i| !deleted_paths.contains(&i.path));
                app.selected.clear();
                app.is_deleting = false;

                // Recalculate stats based on remaining items
                app.stats.total_items = app.items.len() as u64;
                app.stats.total_size = app.items.iter().map(|i| i.size).sum();
                app.stats.items_by_category.clear();
                app.stats.size_by_category.clear();
                for item in &app.items {
                    *app.stats.items_by_category.entry(item.category).or_insert(0) += 1;
                    *app.stats.size_by_category.entry(item.category).or_insert(0) += item.size;
                }

                cx.notify();
            });
        })
        .detach();
    }

    fn get_size_distribution(&self) -> Vec<f64> {
        if self.items.is_empty() {
            return vec![0.0; 10];
        }

        let mut sorted_sizes: Vec<u64> = self.items.iter().map(|i| i.size).collect();
        sorted_sizes.sort();

        let chunk_size = (sorted_sizes.len() / 10).max(1);
        (0..10)
            .map(|i| {
                let start = i * chunk_size;
                let end = (start + chunk_size).min(sorted_sizes.len());
                if start >= sorted_sizes.len() {
                    0.0
                } else {
                    sorted_sizes[start..end].iter().sum::<u64>() as f64
                }
            })
            .collect()
    }

    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let is_scanning = self.is_scanning;
        let scan_progress = self.scan_progress.clone();

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(24.0))
            .py(px(20.0))
            .bg(theme.tokens.primary.opacity(0.05))
            .border_b_1()
            .border_color(theme.tokens.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .size(px(48.0))
                            .rounded_full()
                            .bg(theme.tokens.primary.opacity(0.15))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new("trash-2")
                                    .size(px(24.0))
                                    .color(theme.tokens.primary),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .child(
                                        div()
                                            .text_size(px(28.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.tokens.foreground)
                                            .child("Sweeper"),
                                    )
                                    .child(Badge::new(format!("v{}", env!("CARGO_PKG_VERSION"))).variant(BadgeVariant::Secondary)),
                            )
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(theme.tokens.muted_foreground)
                                    .child("Clean up your disk space with confidence"),
                            ),
                    ),
            )
            .child(if is_scanning {
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .child(Spinner::new())
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme.tokens.foreground)
                                    .child("Scanning..."),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .max_w(px(350.0))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(scan_progress),
                    )
                    .into_any_element()
            } else {
                Button::new("scan", "Rescan Disk")
                    .icon("refresh-cw")
                    .variant(ButtonVariant::Default)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.start_scan(cx);
                    }))
                    .into_any_element()
            })
    }

    fn render_stats(&self) -> impl IntoElement {
        let theme = use_theme();
        let size_distribution = self.get_size_distribution();
        let selection_percent = if self.items.is_empty() {
            0.0
        } else {
            (self.selected.len() as f32 / self.items.len() as f32) * 100.0
        };

        div()
            .flex()
            .gap(px(16.0))
            .px(px(24.0))
            .py(px(16.0))
            .child(
                Card::new()
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.tokens.muted_foreground)
                                            .child("Total Found"),
                                    )
                                    .child(
                                        Icon::new("hard-drive")
                                            .size(px(18.0))
                                            .color(theme.tokens.primary),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(32.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.tokens.foreground)
                                    .child(self.stats.total_size_formatted()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(theme.tokens.muted_foreground)
                                            .child(format!("{} items", self.stats.total_items)),
                                    )
                                    .child(Sparkline::area(size_distribution.clone()).size(
                                        adabraka_ui::components::sparkline::SparklineSize::Sm,
                                    )),
                            ),
                    )
                    .flex_1()
                    .shadow_lg(),
            )
            .child(
                Card::new()
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.tokens.muted_foreground)
                                            .child("Selected"),
                                    )
                                    .child(
                                        Icon::new("check-square")
                                            .size(px(18.0))
                                            .color(theme.tokens.primary),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(32.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(if self.selected.is_empty() {
                                        theme.tokens.muted_foreground
                                    } else {
                                        theme.tokens.primary
                                    })
                                    .child(bytesize::ByteSize(self.selected_size()).to_string()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .text_color(theme.tokens.muted_foreground)
                                            .child(format!("{} items", self.selected.len())),
                                    )
                                    .children(if !self.selected.is_empty() {
                                        Some(
                                            Badge::new(format!("{:.0}%", selection_percent))
                                                .variant(BadgeVariant::Outline),
                                        )
                                    } else {
                                        None
                                    }),
                            ),
                    )
                    .flex_1()
                    .shadow_lg(),
            )
            .child(
                Card::new()
                    .content(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.tokens.muted_foreground)
                                            .child("Scan Time"),
                                    )
                                    .child(
                                        Icon::new("clock")
                                            .size(px(18.0))
                                            .color(theme.tokens.primary),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(32.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.tokens.foreground)
                                    .child(format!(
                                        "{:.1}s",
                                        self.stats.duration_ms as f64 / 1000.0
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.tokens.muted_foreground)
                                    .child(
                                        if self.stats.duration_ms > 0 && self.stats.total_items > 0
                                        {
                                            format!("{} items found", self.stats.total_items)
                                        } else {
                                            "Ready to scan".to_string()
                                        },
                                    ),
                            ),
                    )
                    .flex_1()
                    .shadow_lg(),
            )
    }

    fn render_tabs(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let tabs = [
            FilterTab::All,
            FilterTab::Largest,
            FilterTab::DevArtifacts,
            FilterTab::Caches,
            FilterTab::TempLogs,
            FilterTab::Downloads,
        ];

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(24.0))
            .py(px(12.0))
            .children(tabs.into_iter().map(|tab| {
                let is_active = self.active_tab == tab;
                let bg = if is_active {
                    theme.tokens.primary
                } else {
                    gpui::transparent_black()
                };
                let fg = if is_active {
                    theme.tokens.primary_foreground
                } else {
                    theme.tokens.muted_foreground
                };

                div()
                    .id(SharedString::from(tab.label()))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(14.0))
                    .py(px(8.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(13.0))
                    .font_weight(if is_active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .hover(|s| {
                        s.bg(if is_active {
                            theme.tokens.primary
                        } else {
                            theme.tokens.muted.opacity(0.5)
                        })
                    })
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.active_tab = tab;
                        cx.notify();
                    }))
                    .child(Icon::new(tab.icon()).size(px(14.0)).color(fg))
                    .child(tab.label())
            }))
    }

    fn render_actions(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let has_selection = !self.selected.is_empty();
        let selected_count = self.selected.len();
        let is_deleting = self.is_deleting;
        let filtered_count = self.filtered_items().len();

        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(24.0))
            .py(px(12.0))
            .bg(theme.tokens.muted.opacity(0.3))
            .border_y_1()
            .border_color(theme.tokens.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.tokens.foreground)
                            .child(format!("{} files in view", filtered_count)),
                    )
                    .child(
                        Button::new("select_all", "Select All")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm)
                            .icon("square-check")
                            .disabled(is_deleting || filtered_count == 0)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.select_all(cx);
                            })),
                    )
                    .child(
                        Button::new("deselect", "Clear")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm)
                            .icon("x")
                            .disabled(is_deleting || !has_selection)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.deselect_all(cx);
                            })),
                    ),
            )
            .child(if is_deleting {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(8.0))
                    .bg(theme.tokens.destructive.opacity(0.8))
                    .rounded(px(8.0))
                    .child(Spinner::new())
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.tokens.destructive_foreground)
                            .child("Deleting..."),
                    )
                    .into_any_element()
            } else {
                Button::new("delete", format!("Delete {} items", selected_count))
                    .variant(ButtonVariant::Destructive)
                    .icon("trash-2")
                    .disabled(!has_selection)
                    .on_click(cx.listener(|this, _, _window, cx| {
                        this.show_delete_dialog = true;
                        cx.notify();
                    }))
                    .into_any_element()
            })
    }

    fn render_list(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let items = self.filtered_items();

        if items.is_empty() && !self.is_scanning {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(20.0))
                        .child(
                            div()
                                .size(px(80.0))
                                .rounded_full()
                                .bg(theme.tokens.muted.opacity(0.5))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new("inbox")
                                        .size(px(40.0))
                                        .color(theme.tokens.muted_foreground),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(20.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.tokens.foreground)
                                .child("No items found"),
                        )
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.tokens.muted_foreground)
                                .child("Click 'Rescan Disk' to search for cleanup candidates"),
                        )
                        .child(
                            Button::new("scan_empty", "Start Scanning")
                                .variant(ButtonVariant::Default)
                                .icon("search")
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.start_scan(cx);
                                })),
                        ),
                )
                .into_any_element();
        }

        scrollable_vertical(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .px(px(24.0))
                .py(px(16.0))
                .children(items.into_iter().take(200).map(|item| {
                    let path = item.path.clone();
                    let is_selected = self.selected.contains(&path);
                    let name = item.name.clone();
                    let category = item.category;
                    let category_name = category.name();
                    let path_str = item.path.to_string_lossy().to_string();
                    let size_str = item.size_formatted();
                    let confidence = item.confidence_percent();
                    let is_stale = item.is_stale;

                    let bg = if is_selected {
                        theme.tokens.primary.opacity(0.1)
                    } else {
                        theme.tokens.card
                    };

                    let border_color = if is_selected {
                        theme.tokens.primary.opacity(0.5)
                    } else {
                        theme.tokens.border
                    };

                    let header = if is_stale {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.tokens.foreground)
                                    .child(name),
                            )
                            .child(Badge::new(category_name).variant(BadgeVariant::Secondary))
                            .child(Badge::new("Stale").variant(BadgeVariant::Destructive))
                    } else {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.tokens.foreground)
                                    .child(name),
                            )
                            .child(Badge::new(category_name).variant(BadgeVariant::Secondary))
                    };

                    div()
                        .id(SharedString::from(path_str.clone()))
                        .flex()
                        .items_center()
                        .gap(px(16.0))
                        .px(px(16.0))
                        .py(px(14.0))
                        .bg(bg)
                        .border_1()
                        .border_color(border_color)
                        .rounded(px(8.0))
                        .hover(|s| s.bg(theme.tokens.muted.opacity(0.5)).shadow_md())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.toggle_selection(path.clone(), cx);
                        }))
                        .child(
                            Checkbox::new(SharedString::from(format!("check-{}", path_str)))
                                .checked(is_selected),
                        )
                        .child(
                            div()
                                .size(px(40.0))
                                .rounded(px(8.0))
                                .bg(theme.tokens.muted.opacity(0.5))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(category_icon(category))
                                        .size(px(20.0))
                                        .color(theme.tokens.primary),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .overflow_hidden()
                                .child(header)
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.tokens.muted_foreground)
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(path_str),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .items_end()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_size(px(16.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.tokens.primary)
                                        .child(size_str),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.0))
                                        .child(
                                            Icon::new("gauge")
                                                .size(px(12.0))
                                                .color(theme.tokens.muted_foreground),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(theme.tokens.muted_foreground)
                                                .child(format!("{}%", confidence)),
                                        ),
                                ),
                        )
                })),
        )
        .into_any_element()
    }

    fn render_delete_dialog(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let selected_count = self.selected.len();
        let selected_size = bytesize::ByteSize(self.selected_size()).to_string();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.6))
            .child(
                div()
                    .w(px(440.0))
                    .p(px(28.0))
                    .bg(theme.tokens.card)
                    .border_1()
                    .border_color(theme.tokens.border)
                    .rounded(px(16.0))
                    .shadow_xl()
                    .flex()
                    .flex_col()
                    .gap(px(20.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .child(
                                div()
                                    .size(px(48.0))
                                    .rounded_full()
                                    .bg(theme.tokens.destructive.opacity(0.1))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new("triangle-alert")
                                            .size(px(24.0))
                                            .color(theme.tokens.destructive),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.tokens.foreground)
                                            .child("Confirm Deletion"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(theme.tokens.muted_foreground)
                                            .child("This action will move files to trash"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p(px(16.0))
                            .bg(theme.tokens.muted.opacity(0.3))
                            .rounded(px(8.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .child(
                                        Icon::new("files")
                                            .size(px(20.0))
                                            .color(theme.tokens.muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .text_color(theme.tokens.foreground)
                                            .child(format!("{} items selected", selected_count)),
                                    ),
                            )
                            .child(Badge::new(selected_size).variant(BadgeVariant::Outline)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(12.0))
                            .child(
                                Button::new("cancel", "Cancel")
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.show_delete_dialog = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("confirm_delete", "Delete Files")
                                    .variant(ButtonVariant::Destructive)
                                    .icon("trash-2")
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.delete_selected(cx);
                                    })),
                            ),
                    ),
            )
    }
}

impl Render for SweeperApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme();
        let show_dialog = self.show_delete_dialog;

        let mut container = div()
            .size_full()
            .bg(theme.tokens.background)
            .flex()
            .flex_col()
            .relative()
            .child(self.render_header(cx))
            .child(self.render_stats())
            .child(self.render_tabs(cx))
            .child(self.render_actions(cx))
            .child(div().flex_1().overflow_hidden().child(self.render_list(cx)));

        if show_dialog {
            container = container.child(self.render_delete_dialog(cx));
        }

        container
    }
}
