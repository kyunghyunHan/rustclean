#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]
use std::sync::Arc;

use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use egui::FontFamily;
use egui::FontDefinitions;
use egui::FontData;
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("MacSweep - Cache Cleaner")
            .with_min_inner_size([1000.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "MacSweep",
        options,
        Box::new(|cc| {
            // 다크 모드 설정
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            
            // 한국어 폰트 설정
            setup_fonts(&cc.egui_ctx);
            
            Ok(Box::<CacheCleanerApp>::default())
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    
    // 한국어 폰트 데이터 추가
    // 실제 폰트 파일 경로: assets/korean_font.ttf
    
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "nanum_gothic".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/NanumGothic-Bold.ttf")).into(),
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "nanum_gothic".to_owned());

    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "nanum_gothic".to_owned());


    
    ctx.set_fonts(fonts);
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CacheType {
    System,
    User,
    Developer,
    Browser,
    Application,
}

impl CacheType {
    fn color(&self) -> egui::Color32 {
        match self {
            CacheType::System => egui::Color32::from_rgb(255, 107, 107),     // 빨강
            CacheType::User => egui::Color32::from_rgb(116, 185, 255),       // 파랑
            CacheType::Developer => egui::Color32::from_rgb(162, 255, 178),  // 초록
            CacheType::Browser => egui::Color32::from_rgb(255, 177, 66),     // 주황
            CacheType::Application => egui::Color32::from_rgb(186, 85, 211), // 보라
        }
    }
    
    fn icon(&self) -> &'static str {
        match self {
            CacheType::System => "⚙️",
            CacheType::User => "👤",
            CacheType::Developer => "💻",
            CacheType::Browser => "🌐",
            CacheType::Application => "📱",
        }
    }
}

#[derive(PartialEq)]
enum AppState {
    Scanning,
    Ready,
    Cleaning,
    Complete,
}

struct ScanProgress {
    current_path: String,
    scanned_items: usize,
    total_size: u64,
    progress: f32,
}

struct CleaningStats {
    items_cleaned: usize,
    bytes_freed: u64,
    time_taken: f32,
    errors: Vec<String>,
}

struct CacheCleanerApp {
    state: AppState,
    cache_items: Vec<CacheItem>,
    filtered_items: Vec<CacheItem>,
    scan_progress: ScanProgress,
    cleaning_stats: CleaningStats,
    filter_text: String,
    selected_types: HashMap<CacheType, bool>,
    show_unsafe: bool,
    sort_by_size: bool,
    auto_select_safe: bool,
    dry_run: bool,
    last_scan_time: Option<SystemTime>,
}

impl Default for CacheCleanerApp {
    fn default() -> Self {
        let mut selected_types = HashMap::new();
        selected_types.insert(CacheType::User, true);
        selected_types.insert(CacheType::Developer, true);
        selected_types.insert(CacheType::Browser, true);
        selected_types.insert(CacheType::Application, true);
        selected_types.insert(CacheType::System, false); // 기본적으로 시스템 캐시는 비활성화
        
        let mut app = Self {
            state: AppState::Ready,
            cache_items: Vec::new(),
            filtered_items: Vec::new(),
            scan_progress: ScanProgress {
                current_path: String::new(),
                scanned_items: 0,
                total_size: 0,
                progress: 0.0,
            },
            cleaning_stats: CleaningStats {
                items_cleaned: 0,
                bytes_freed: 0,
                time_taken: 0.0,
                errors: Vec::new(),
            },
            filter_text: String::new(),
            selected_types,
            show_unsafe: false,
            sort_by_size: true,
            auto_select_safe: true,
            dry_run: false,
            last_scan_time: None,
        };
        
        // 샘플 데이터 생성 (실제 구현에서는 실제 스캔 로직으로 대체)
        app.generate_sample_data();
        app.update_filtered_items();
        app
    }
}

impl CacheCleanerApp {
    fn generate_sample_data(&mut self) {
        // 실제 macOS 캐시 경로들을 시뮬레이션
        let sample_caches = vec![
            ("Safari/WebKit", CacheType::Browser, 245_000_000, true, "Safari 웹 캐시"),
            ("Chrome/Default/Cache", CacheType::Browser, 890_000_000, true, "Chrome 브라우저 캐시"),
            ("Xcode/DerivedData", CacheType::Developer, 1_200_000_000, true, "Xcode 빌드 캐시"),
            ("com.apple.dt.Xcode", CacheType::Developer, 450_000_000, true, "Xcode 도구 캐시"),
            ("npm/_cacache", CacheType::Developer, 320_000_000, true, "NPM 패키지 캐시"),
            ("Homebrew/downloads", CacheType::Developer, 180_000_000, true, "Homebrew 다운로드"),
            ("cargo/registry/cache", CacheType::Developer, 567_000_000, true, "Rust Cargo 캐시"),
            ("com.apple.Safari", CacheType::User, 125_000_000, true, "Safari 사용자 캐시"),
            ("com.spotify.client", CacheType::Application, 234_000_000, true, "Spotify 캐시"),
            ("com.adobe.Creative Cloud", CacheType::Application, 678_000_000, true, "Adobe 크리에이티브 클라우드"),
            ("com.apple.security", CacheType::System, 45_000_000, false, "보안 시스템 캐시"),
            ("CloudKit", CacheType::System, 23_000_000, false, "iCloud 동기화 캐시"),
            ("Logs/DiagnosticReports", CacheType::System, 89_000_000, true, "시스템 진단 로그"),
            ("Firefox/Profiles/default", CacheType::Browser, 412_000_000, true, "Firefox 프로필 캐시"),
            ("VS Code/logs", CacheType::Developer, 67_000_000, true, "VS Code 로그"),
            ("com.docker.docker", CacheType::Developer, 2_100_000_000, true, "Docker 이미지 캐시"),
        ];
        
        for (i, (name, cache_type, size, is_safe, description)) in sample_caches.iter().enumerate() {
            let path = PathBuf::from(format!("/Users/user/Library/Caches/{}", name));
            let last_modified = SystemTime::now()
                .checked_sub(std::time::Duration::from_secs((i * 3600 + 60) as u64))
                .unwrap_or(SystemTime::now());
            
            self.cache_items.push(CacheItem {
                path,
                name: name.to_string(),
                size: *size,
                last_modified,
                item_type: cache_type.clone(),
                is_selected: *is_safe && self.auto_select_safe,
                is_safe: *is_safe,
                description: description.to_string(),
            });
        }
        
        self.last_scan_time = Some(SystemTime::now());
    }
    
    fn update_filtered_items(&mut self) {
        self.filtered_items = self.cache_items
            .iter()
            .filter(|item| {
                // 타입 필터
                if !self.selected_types.get(&item.item_type).unwrap_or(&false) {
                    return false;
                }
                
                // 안전성 필터
                if !self.show_unsafe && !item.is_safe {
                    return false;
                }
                
                // 텍스트 필터
                if !self.filter_text.is_empty() {
                    let search_text = self.filter_text.to_lowercase();
                    if !item.name.to_lowercase().contains(&search_text) 
                        && !item.description.to_lowercase().contains(&search_text) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // 정렬
        if self.sort_by_size {
            self.filtered_items.sort_by(|a, b| b.size.cmp(&a.size));
        } else {
            self.filtered_items.sort_by(|a, b| a.name.cmp(&b.name));
        }
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
    
    fn total_selected_size(&self) -> u64 {
        self.filtered_items
            .iter()
            .filter(|item| item.is_selected)
            .map(|item| item.size)
            .sum()
    }
    
    fn selected_count(&self) -> usize {
        self.filtered_items
            .iter()
            .filter(|item| item.is_selected)
            .count()
    }
    
    fn start_scan(&mut self) {
        self.state = AppState::Scanning;
        self.scan_progress = ScanProgress {
            current_path: "시작 중...".to_string(),
            scanned_items: 0,
            total_size: 0,
            progress: 0.0,
        };
        // 실제 구현에서는 여기서 비동기 스캔 시작
    }
    
    fn start_cleaning(&mut self) {
        self.state = AppState::Cleaning;
        self.cleaning_stats = CleaningStats {
            items_cleaned: 0,
            bytes_freed: 0,
            time_taken: 0.0,
            errors: Vec::new(),
        };
        // 실제 구현에서는 여기서 비동기 클리닝 시작
    }
}

impl eframe::App for CacheCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 상단 메뉴바

        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                
                // 로고와 타이틀
                ui.label(egui::RichText::new("🧹 MacSweep")
                    .size(24.0)
                    .color(egui::Color32::from_rgb(100, 200, 255)));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 상태 표시
                    let (status_text, status_color) = match self.state {
                        AppState::Ready => ("준비됨", egui::Color32::GREEN),
                        AppState::Scanning => ("스캔 중...", egui::Color32::YELLOW),
                        AppState::Cleaning => ("정리 중...", egui::Color32::ORANGE),
                        AppState::Complete => ("완료", egui::Color32::GREEN),
                    };
                    
                    ui.colored_label(status_color, egui::RichText::new(status_text).size(14.0));
                    
                    if let Some(scan_time) = self.last_scan_time {
                        let elapsed = scan_time.elapsed().unwrap_or_default().as_secs();
                        ui.label(format!("마지막 스캔: {}초 전", elapsed));
                    }
                });
            });
            ui.add_space(5.0);
        });
        
        // 하단 액션 패널
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                
                // 선택된 항목 정보
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("선택됨:");
                        ui.colored_label(
                            egui::Color32::LIGHT_BLUE, 
                            format!("{} 항목", self.selected_count())
                        );
                        ui.separator();
                        ui.label("용량:");
                        ui.colored_label(
                            egui::Color32::ORANGE, 
                            Self::format_size(self.total_selected_size())
                        );
                    });
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 주요 액션 버튼들
                    let can_clean = self.selected_count() > 0 && matches!(self.state, AppState::Ready);
                    let can_scan = matches!(self.state, AppState::Ready);
                    
                    if ui.add_enabled(can_clean, 
                        egui::Button::new(if self.dry_run { "🔍 테스트 실행" } else { "🗑️ 정리 시작" })
                            .fill(if self.dry_run { egui::Color32::from_rgb(100, 150, 200) } else { egui::Color32::from_rgb(220, 100, 100) })
                            .min_size([120.0, 35.0].into())
                    ).clicked() {
                        self.start_cleaning();
                    }
                    
                    if ui.add_enabled(can_scan,
                        egui::Button::new("🔄 다시 스캔")
                            .fill(egui::Color32::from_rgb(100, 200, 150))
                            .min_size([100.0, 35.0].into())
                    ).clicked() {
                        self.start_scan();
                    }
                    
                    ui.checkbox(&mut self.dry_run, "테스트 모드");
                });
            });
            ui.add_space(10.0);
        });
        
        // 메인 컨텐츠 영역
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state {
                AppState::Scanning => {
                    // 스캔 진행률 화면
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            
                            ui.label(egui::RichText::new("🔍 캐시 파일 스캔 중...")
                                .size(24.0)
                                .color(egui::Color32::LIGHT_BLUE));
                            
                            ui.add_space(20.0);
                            
                            ui.add(egui::ProgressBar::new(self.scan_progress.progress)
                                .desired_width(400.0)
                                .show_percentage());
                            
                            ui.add_space(10.0);
                            ui.label(&self.scan_progress.current_path);
                            ui.label(format!("발견된 항목: {}", self.scan_progress.scanned_items));
                            ui.label(format!("총 크기: {}", Self::format_size(self.scan_progress.total_size)));
                        });
                    });
                },
                
                AppState::Cleaning => {
                    // 클리닝 진행률 화면
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(50.0);
                            
                            ui.label(egui::RichText::new(if self.dry_run { "🔍 테스트 실행 중..." } else { "🗑️ 캐시 정리 중..." })
                                .size(24.0)
                                .color(if self.dry_run { egui::Color32::LIGHT_BLUE } else { egui::Color32::ORANGE }));
                            
                            ui.add_space(20.0);
                            
                            ui.add(egui::ProgressBar::new(0.7) // 임시 진행률
                                .desired_width(400.0)
                                .show_percentage());
                            
                            ui.add_space(10.0);
                            ui.label(format!("처리된 항목: {}", self.cleaning_stats.items_cleaned));
                            ui.label(format!("확보된 용량: {}", Self::format_size(self.cleaning_stats.bytes_freed)));
                        });
                    });
                },
                
                _ => {
                    // 메인 인터페이스
                    ui.horizontal(|ui| {
                        // 왼쪽 사이드바 - 필터 및 옵션
                        ui.vertical(|ui| {
                            ui.set_width(280.0);
                            
                            // 검색 박스
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong("🔍 검색");
                                    ui.add(egui::TextEdit::singleline(&mut self.filter_text)
                                        .hint_text("이름 또는 설명으로 검색...")
                                        .desired_width(250.0));
                                });
                            });
                            
                            ui.add_space(10.0);
                            
                            // 캐시 타입 필터
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong("📁 캐시 유형");
                                    
                                    for cache_type in [CacheType::User, CacheType::Developer, CacheType::Browser, CacheType::Application, CacheType::System] {
                                        ui.horizontal(|ui| {
                                            let is_enabled = self.selected_types.get(&cache_type).copied().unwrap_or(false);
                                            let mut enabled = is_enabled;
                                            
                                            ui.label(cache_type.icon());
                                            if ui.checkbox(&mut enabled, match cache_type {
                                                CacheType::System => "시스템",
                                                CacheType::User => "사용자",
                                                CacheType::Developer => "개발도구",
                                                CacheType::Browser => "브라우저",
                                                CacheType::Application => "앱",
                                            }).changed() {
                                                self.selected_types.insert(cache_type.clone(), enabled);
                                                self.update_filtered_items();
                                            }
                                            
                                            // 아이템 개수 표시
                                            let count = self.cache_items.iter()
                                                .filter(|item| item.item_type == cache_type)
                                                .count();
                                            if count > 0 {
                                                ui.small(format!("({})", count));
                                            }
                                        });
                                    }
                                });
                            });
                            
                            ui.add_space(10.0);
                            
                            // 옵션
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong("⚙️ 옵션");
                                    
                                    if ui.checkbox(&mut self.show_unsafe, "위험한 항목 표시").changed() {
                                        self.update_filtered_items();
                                    }
                                    
                                    if ui.checkbox(&mut self.sort_by_size, "크기순 정렬").changed() {
                                        self.update_filtered_items();
                                    }
                                    
                                    ui.checkbox(&mut self.auto_select_safe, "안전한 항목 자동 선택");
                                    
                                    ui.separator();
                                    
                                    // 빠른 액션
                                    if ui.button("🔘 모두 선택").clicked() {
                                        for i in 0..self.filtered_items.len() {
                                            self.filtered_items[i].is_selected = true;
                                        }
                                    }
                                    
                                    if ui.button("⭕ 모두 해제").clicked() {
                                        for i in 0..self.filtered_items.len() {
                                            self.filtered_items[i].is_selected = false;
                                        }
                                    }
                                    
                                    if ui.button("✅ 안전한 것만").clicked() {
                                        for i in 0..self.filtered_items.len() {
                                            self.filtered_items[i].is_selected = self.filtered_items[i].is_safe;
                                        }
                                    }
                                });
                            });
                        });
                        
                        ui.separator();
                        
                        // 메인 캐시 목록
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading("캐시 파일 목록");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(format!("{} / {} 항목", 
                                        self.filtered_items.len(), 
                                        self.cache_items.len()));
                                });
                            });
                            
                            ui.separator();
                            
                            // 캐시 목록 스크롤 영역
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let filtered_items_len = self.filtered_items.len();
                                    
                                    for i in 0..filtered_items_len {
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                // 체크박스 - 분리된 변수로 처리
                                                let mut is_selected = self.filtered_items[i].is_selected;
                                                if ui.checkbox(&mut is_selected, "").changed() {
                                                    self.filtered_items[i].is_selected = is_selected;
                                                }
                                                
                                                // 타입 아이콘과 색상
                                                ui.colored_label(
                                                    self.filtered_items[i].item_type.color(), 
                                                    self.filtered_items[i].item_type.icon()
                                                );
                                                
                                                ui.vertical(|ui| {
                                                    // 이름
                                                    ui.horizontal(|ui| {
                                                        ui.strong(&self.filtered_items[i].name);
                                                        if !self.filtered_items[i].is_safe {
                                                            ui.colored_label(egui::Color32::RED, "⚠️");
                                                        }
                                                    });
                                                    
                                                    // 설명
                                                    ui.small(&self.filtered_items[i].description);
                                                    
                                                    // 경로 (작게)
                                                    ui.small(format!("📁 {}", self.filtered_items[i].path.display()));
                                                });
                                                
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    // 크기
                                                    ui.vertical(|ui| {
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.strong(Self::format_size(self.filtered_items[i].size));
                                                            
                                                            // 수정 시간
                                                            if let Ok(elapsed) = self.filtered_items[i].last_modified.elapsed() {
                                                                let days = elapsed.as_secs() / 86400;
                                                                ui.small(if days == 0 {
                                                                    "오늘".to_string()
                                                                } else {
                                                                    format!("{}일 전", days)
                                                                });
                                                            }
                                                        });
                                                    });
                                                });
                                            });
                                        });
                                        
                                        if i < filtered_items_len - 1 {
                                            ui.add_space(5.0);
                                        }
                                    }
                                    
                                    if self.filtered_items.is_empty() {
                                        ui.centered_and_justified(|ui| {
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(50.0);
                                                ui.label("표시할 캐시 항목이 없습니다");
                                                ui.small("필터 설정을 확인해보세요");
                                            });
                                        });
                                    }
                                });
                        });
                    });
                }
            }
        });
        
        // 시뮬레이션용 상태 변경 (실제 구현에서는 제거)
        if matches!(self.state, AppState::Scanning) {
            self.scan_progress.progress = (self.scan_progress.progress + 0.01).min(1.0);
            if self.scan_progress.progress >= 1.0 {
                self.state = AppState::Ready;
                self.generate_sample_data(); // 새로운 데이터 생성
                self.update_filtered_items();
            }
            ctx.request_repaint();
        }
        
        if matches!(self.state, AppState::Cleaning) {
            self.cleaning_stats.items_cleaned += 1;
            self.cleaning_stats.bytes_freed += 1000000;
            if self.cleaning_stats.items_cleaned >= 10 {
                self.state = AppState::Complete;
            }
            ctx.request_repaint();
        }
    }
}