#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dirs::home_dir;
use gpui::{
    App, Bounds, Context, MouseButton, Window, WindowBounds, WindowOptions, div, px, rgb, size,
    white, black, prelude::*,
};
use gpui_platform::application;
use sysinfo::System;
use walkdir::WalkDir;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CacheType {
    User,
    Developer,
    Browser,
    Application,
    System,
}

impl CacheType {
    fn label(&self) -> &'static str {
        match self {
            CacheType::User => "User",
            CacheType::Developer => "Developer",
            CacheType::Browser => "Browser",
            CacheType::Application => "Application",
            CacheType::System => "System",
        }
    }
}

#[derive(Clone, Debug)]
struct CacheItem {
    path: PathBuf,
    name: String,
    size: u64,
    last_modified: SystemTime,
    item_type: CacheType,
    is_selected: bool,
    is_safe: bool,
    description: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppState {
    Idle,
    Scanning,
    Cleaning,
    Complete,
}

struct MemoryInfo {
    total: u64,
    used: u64,
}

struct CleaningStats {
    items_cleaned: usize,
    bytes_freed: u64,
    errors: Vec<String>,
}

struct CacheCleanerView {
    state: AppState,
    cache_items: Vec<CacheItem>,
    filtered_indices: Vec<usize>,
    selected_types: HashMap<CacheType, bool>,
    show_unsafe: bool,
    sort_by_size: bool,
    auto_select_safe: bool,
    dry_run: bool,
    last_scan_time: Option<SystemTime>,
    memory: MemoryInfo,
    cleaning_stats: CleaningStats,
}

impl CacheCleanerView {
    fn new(_cx: &mut Context<Self>) -> Self {
        let mut selected_types = HashMap::new();
        selected_types.insert(CacheType::User, true);
        selected_types.insert(CacheType::Developer, true);
        selected_types.insert(CacheType::Browser, true);
        selected_types.insert(CacheType::Application, true);
        selected_types.insert(CacheType::System, false);

        let mut view = Self {
            state: AppState::Idle,
            cache_items: Vec::new(),
            filtered_indices: Vec::new(),
            selected_types,
            show_unsafe: false,
            sort_by_size: true,
            auto_select_safe: true,
            dry_run: true,
            last_scan_time: None,
            memory: MemoryInfo { total: 0, used: 0 },
            cleaning_stats: CleaningStats {
                items_cleaned: 0,
                bytes_freed: 0,
                errors: Vec::new(),
            },
        };

        view.refresh_memory();
        view.scan_caches();
        view
    }

    fn refresh_memory(&mut self) {
        let mut system = System::new_all();
        system.refresh_memory();
        self.memory = MemoryInfo {
            total: system.total_memory(),
            used: system.used_memory(),
        };
    }

    fn scan_caches(&mut self) {
        self.state = AppState::Scanning;
        self.cache_items.clear();

        let cache_root = match home_dir() {
            Some(home) => home.join("Library/Caches"),
            None => {
                self.state = AppState::Idle;
                return;
            }
        };

        if cache_root.exists() {
            if let Ok(entries) = std::fs::read_dir(&cache_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("(unknown)")
                        .to_string();

                    let size = dir_size(&path);
                    let last_modified = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::now());

                    let item_type = classify_cache(&name);
                    let is_safe = item_type != CacheType::System;

                    self.cache_items.push(CacheItem {
                        path: path.clone(),
                        name,
                        size,
                        last_modified,
                        item_type,
                        is_selected: is_safe && self.auto_select_safe,
                        is_safe,
                        description: "macOS cache directory".to_string(),
                    });
                }
            }
        }

        self.last_scan_time = Some(SystemTime::now());
        self.update_filtered_indices();
        self.refresh_memory();
        self.state = AppState::Idle;
    }

    fn update_filtered_indices(&mut self) {
        self.filtered_indices = self
            .cache_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if !self.selected_types.get(&item.item_type).copied().unwrap_or(false) {
                    return false;
                }
                if !self.show_unsafe && !item.is_safe {
                    return false;
                }
                true
            })
            .map(|(index, _)| index)
            .collect();

        if self.sort_by_size {
            self.filtered_indices.sort_by(|a, b| {
                let size_a = self.cache_items[*a].size;
                let size_b = self.cache_items[*b].size;
                size_b.cmp(&size_a)
            });
        } else {
            self.filtered_indices.sort_by(|a, b| {
                let name_a = &self.cache_items[*a].name;
                let name_b = &self.cache_items[*b].name;
                name_a.cmp(name_b)
            });
        }
    }

    fn total_selected_size(&self) -> u64 {
        self.filtered_indices
            .iter()
            .filter_map(|&index| self.cache_items.get(index))
            .filter(|item| item.is_selected)
            .map(|item| item.size)
            .sum()
    }

    fn selected_count(&self) -> usize {
        self.filtered_indices
            .iter()
            .filter_map(|&index| self.cache_items.get(index))
            .filter(|item| item.is_selected)
            .count()
    }

    fn clean_selected(&mut self) {
        self.state = AppState::Cleaning;
        self.cleaning_stats = CleaningStats {
            items_cleaned: 0,
            bytes_freed: 0,
            errors: Vec::new(),
        };

        let indices: Vec<usize> = self.filtered_indices.clone();
        for index in indices {
            let Some(item) = self.cache_items.get_mut(index) else {
                continue;
            };

            if !item.is_selected {
                continue;
            }

            if self.dry_run {
                self.cleaning_stats.items_cleaned += 1;
                self.cleaning_stats.bytes_freed += item.size;
                continue;
            }

            match std::fs::remove_dir_all(&item.path) {
                Ok(()) => {
                    self.cleaning_stats.items_cleaned += 1;
                    self.cleaning_stats.bytes_freed += item.size;
                }
                Err(err) => {
                    self.cleaning_stats
                        .errors
                        .push(format!("{}: {}", item.path.display(), err));
                }
            }
        }

        self.scan_caches();
        self.state = AppState::Complete;
    }
}

impl Render for CacheCleanerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = match self.state {
            AppState::Idle => "Idle",
            AppState::Scanning => "Scanning",
            AppState::Cleaning => "Cleaning",
            AppState::Complete => "Complete",
        };

        let memory_text = format!(
            "Memory: {} / {}",
            format_size(self.memory.used * 1024),
            format_size(self.memory.total * 1024)
        );

        let scan_time_text = self
            .last_scan_time
            .and_then(|time| time.elapsed().ok())
            .map(|elapsed| format!("Last scan: {}s ago", elapsed.as_secs()))
            .unwrap_or_else(|| "Last scan: -".to_string());

        let clean_text = if self.dry_run {
            "Dry run (no delete)"
        } else {
            "Clean selected"
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p(px(16.0))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(format!("MacSweep Memory Cleaner"))
                    .child(format!("Status: {status}"))
                    .child(scan_time_text),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(memory_text)
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0xdddddd))
                            .border_1()
                            .border_color(black())
                            .child("Refresh Memory")
                            .hover(|style| style.cursor_pointer())
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _e, _w, cx| {
                                this.refresh_memory();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0xdddddd))
                            .border_1()
                            .border_color(black())
                            .child("Scan Caches")
                            .hover(|style| style.cursor_pointer())
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _e, _w, cx| {
                                this.scan_caches();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0xdddddd))
                            .border_1()
                            .border_color(black())
                            .child(clean_text)
                            .hover(|style| style.cursor_pointer())
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _e, _w, cx| {
                                this.clean_selected();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .bg(rgb(0xeeeeee))
                            .border_1()
                            .border_color(black())
                            .child(if self.dry_run { "Dry run: ON" } else { "Dry run: OFF" })
                            .hover(|style| style.cursor_pointer())
                            .on_mouse_up(MouseButton::Left, cx.listener(|this, _e, _w, cx| {
                                this.dry_run = !this.dry_run;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(format!(
                        "Selected: {} items / {}",
                        self.selected_count(),
                        format_size(self.total_selected_size())
                    ))
                    .child(format!(
                        "Cleaned: {} items / {}",
                        self.cleaning_stats.items_cleaned,
                        format_size(self.cleaning_stats.bytes_freed)
                    )),
            )
            .child(
                div()
                    .border_1()
                    .border_color(black())
                    .p(px(8.0))
                    .bg(white())
                    .children(
                        self.filtered_indices
                            .iter()
                            .filter_map(|&index| self.cache_items.get(index))
                            .map(|item| {
                                let item_name = item.name.clone();
                                let item_size = format_size(item.size);
                                let item_path = item.path.clone();
                                let item_path_label = item.path.display().to_string();
                                let selected = item.is_selected;

                                div()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .child(if selected { "[x]" } else { "[ ]" })
                                    .child(item_name.clone())
                                    .child(item_size)
                                    .child(item_path_label)
                                    .hover(|style| style.cursor_pointer())
                                    .on_mouse_up(MouseButton::Left, cx.listener(
                                        move |this, _e, _w, cx| {
                                            if let Some(found) = this
                                                .cache_items
                                                .iter_mut()
                                                .find(|entry| entry.path == item_path)
                                            {
                                                found.is_selected = !found.is_selected;
                                                cx.notify();
                                            }
                                        },
                                    ))
                            }),
                    ),
            )
            .children(
                self.cleaning_stats
                    .errors
                    .iter()
                    .map(|err| format!("Error: {}", err)),
            )
    }
}

fn classify_cache(name: &str) -> CacheType {
    let lower = name.to_lowercase();
    if lower.contains("safari") || lower.contains("chrome") || lower.contains("firefox") {
        CacheType::Browser
    } else if lower.contains("xcode") || lower.contains("cargo") || lower.contains("npm") {
        CacheType::Developer
    } else if lower.contains("com.apple") || lower.contains("system") {
        CacheType::System
    } else if lower.contains("com.") {
        CacheType::Application
    } else {
        CacheType::User
    }
}

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .sum()
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| CacheCleanerView::new(cx)),
        )
        .unwrap();
        cx.activate(true);
    });
}
