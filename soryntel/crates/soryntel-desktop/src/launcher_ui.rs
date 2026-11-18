//! Soryntel Chemistry launcher.
//! White branded header bar (Settings gear + centered title + metadata),
//! Industrial atmospheric backdrop painted via `egui::Painter`, 4 left showcase cards,
//! 6×4 translucent glass tile matrix with active green indicators, Sub Menu & footer bar.

use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShowcaseCategory {
    None,
    Products,
    Services,
    Sustainability,
    ResearchDev,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TileAction {
    Launch(&'static str),
    Help,
    SubMenu,
}

#[derive(Clone, Copy)]
pub struct TileSpec {
    pub code: &'static str,
    pub label: &'static str,
    pub action: TileAction,
    pub category: ShowcaseCategory,
    pub running: bool,
}

pub struct LauncherUiState {
    logo: Option<egui::TextureHandle>,
    pub detailed_view: bool,
    pub selected_category: ShowcaseCategory,
    pub cards_category: ShowcaseCategory,
    pub cards_recent_only: bool,
    pub cards_search: String,
    pub show_settings: bool,
    pub show_help: bool,
    pub show_submenu: bool,
}

impl Default for LauncherUiState {
    fn default() -> Self {
        Self {
            logo: None,
            detailed_view: false,
            selected_category: ShowcaseCategory::None,
            cards_category: ShowcaseCategory::None,
            cards_recent_only: false,
            cards_search: String::new(),
            show_settings: false,
            show_help: false,
            show_submenu: false,
        }
    }
}

pub enum LauncherCommand {
    None,
    OpenModule(&'static str),
    KillAll,
}

pub fn render_hsc10_header(
    root_ui: &mut egui::Ui,
    ui_state: &mut LauncherUiState,
    species_count: usize,
) {
    egui::Panel::top("hsc10_launcher_header")
        .frame(
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(245, 247, 250))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(214, 220, 229)))
                .inner_margin(egui::Margin::symmetric(14, 6)),
        )
        .show(root_ui, |ui| {
            ui.set_height(38.0);
            let rect = ui.max_rect();

            ui.horizontal(|ui| {
                // Left gear icon button matching Fig. 1
                let (gear_rect, gear_resp) =
                    ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
                paint_gear_button(ui.painter(), gear_rect, gear_resp.hovered());
                if gear_resp
                    .on_hover_text(crate::i18n::text("nav.000"))
                    .clicked()
                {
                    ui_state.show_settings = !ui_state.show_settings;
                }

                // Right metadata block matching Fig. 1 top-right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let meta_col = egui::Color32::from_rgb(100, 112, 128);
                        ui.label(
                            egui::RichText::new(concat!("Soryntel ", env!("CARGO_PKG_VERSION")))
                                .size(9.5)
                                .color(meta_col),
                        );
                        ui.label(
                            egui::RichText::new(crate::i18n::text("nav.001"))
                                .size(9.5)
                                .color(meta_col),
                        );
                        ui.label(
                            egui::RichText::new(format!("Catálogo: {species_count} especies"))
                                .size(9.5)
                                .color(meta_col),
                        );
                    });
                });
            });

            let logo = ui_state.logo.get_or_insert_with(|| {
                let logo = eframe::icon_data::from_png_bytes(include_bytes!(
                    "../../../packaging/soryntel-logo.png"
                ))
                .expect("el logotipo incluido debe ser un PNG válido");
                ui.ctx().load_texture(
                    "soryntel-logo",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [logo.width as usize, logo.height as usize],
                        &logo.rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
            let logo_size = logo.size_vec2() * (36.0 / logo.size_vec2().y);
            let logo_rect = egui::Rect::from_center_size(rect.center(), logo_size);
            ui.painter().image(
                logo.id(),
                logo_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        });
}

pub fn render_hsc10_footer(
    root_ui: &mut egui::Ui,
    ui_state: &mut LauncherUiState,
    open_count: usize,
) -> LauncherCommand {
    let mut cmd = LauncherCommand::None;
    egui::Panel::bottom("hsc10_launcher_footer")
        .frame(
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(245, 247, 250))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(214, 220, 229)))
                .inner_margin(egui::Margin::symmetric(14, 5)),
        )
        .show(root_ui, |ui| {
            ui.set_height(30.0);
            ui.horizontal_wrapped(|ui| {
                // Orange RSS-style badge + release status line matching Fig. 1
                let (rss_rect, _) =
                    ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(rss_rect, 3.0, egui::Color32::from_rgb(234, 138, 46));
                p.circle_filled(rss_rect.left_bottom() + egui::vec2(5.0, -5.0), 1.8, egui::Color32::WHITE);
                p.circle_stroke(
                    rss_rect.left_bottom() + egui::vec2(5.0, -5.0),
                    5.5,
                    egui::Stroke::new(1.3, egui::Color32::WHITE),
                );

                ui.add_space(4.0);
                let status = if open_count == 0 {
                    "Soryntel Chemistry · 20 módulos disponibles".to_string()
                } else {
                    format!("Ventanas de módulos abiertas: {open_count}")
                };
                ui.label(
                    egui::RichText::new(status)
                        .italics()
                        .strong()
                        .size(11.0)
                        .color(egui::Color32::from_rgb(30, 41, 59)),
                );

                ui.add_space(10.0);
                let view_lbl = if ui_state.detailed_view {
                    crate::i18n::text("nav.002")
                } else {
                    crate::i18n::text("nav.003")
                };
                if ui.small_button(view_lbl).clicked() {
                    ui_state.detailed_view = !ui_state.detailed_view;
                    ui_state.selected_category = ShowcaseCategory::None;
                }

                if open_count > 0
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Cerrar todas las ventanas")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(185, 28, 28)),
                            )
                            .fill(egui::Color32::from_rgb(254, 226, 226))
                            .corner_radius(4),
                        )
                        .clicked()
                {
                    cmd = LauncherCommand::KillAll;
                }
            });
            ui.label(
                egui::RichText::new(rust_i18n::t!("nav.sorynteldb_attribution").as_ref())
                    .size(11.0)
                    .color(egui::Color32::from_rgb(71, 85, 105)),
            );
        });
    cmd
}

pub fn render_hsc10_grid_canvas(
    root_ui: &mut egui::Ui,
    ui_state: &mut LauncherUiState,
    tiles: &[TileSpec; 24],
    species_count: usize,
) -> LauncherCommand {
        let ctx = &root_ui.ctx().clone();
    let mut cmd = LauncherCommand::None;

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(10, 32, 64)))
        .show(root_ui, |ui| {
            let canvas = ui.max_rect();
            paint_industrial_backdrop(ui.painter(), canvas);

            let compact = ui.available_width() < 880.0;
            let columns = if compact { 4 } else { 6 };
            let scale = (ui.available_width() / 1000.0).clamp(0.85, 1.12);
            let card_size = egui::vec2(196.0, 104.0) * scale;
            let tile_width = if compact {
                ((ui.available_width() - 76.0) / columns as f32).min(160.0)
            } else {
                112.0 * scale
            };
            let tile_size = egui::vec2(tile_width, 102.0 * scale);
            let gap = 10.0;
            let categories = [
                (ShowcaseCategory::Products, crate::i18n::text("nav.004")),
                (ShowcaseCategory::Services, crate::i18n::text("nav.005")),
                (ShowcaseCategory::Sustainability, crate::i18n::text("nav.006")),
                (ShowcaseCategory::ResearchDev, crate::i18n::text("nav.007")),
            ];

            ui.spacing_mut().scroll.floating = false;
            egui::ScrollArea::vertical()
                .id_salt("launcher_module_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        if compact {
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(14.0);
                                for (category, title) in categories {
                                    let active = ui_state.selected_category == category;
                                    if ui.selectable_label(active, title).clicked() {
                                        toggle_category(ui_state, category);
                                    }
                                }
                            });
                            ui.add_space(8.0);
                        }

                        let grid_width = columns as f32 * tile_size.x + (columns - 1) as f32 * gap;
                        let content_width = grid_width
                            + if compact { 0.0 } else { card_size.x + 22.0 };
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.add_space(((ui.available_width() - content_width) * 0.5).max(10.0));
                            if !compact {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = gap;
                                    for (category, title) in categories {
                                        let active = ui_state.selected_category == category;
                                        let (rect, response) =
                                            ui.allocate_exact_size(card_size, egui::Sense::click());
                                        paint_showcase_card(
                                            ui.painter(), rect, category, title, active, response.hovered(),
                                        );
                                        if response.clicked() {
                                            toggle_category(ui_state, category);
                                        }
                                    }
                                });
                                ui.add_space(22.0);
                            }
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = gap;
                                for row in tiles.chunks(columns) {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = gap;
                                        for tile in row {
                                            let dimmed = ui_state.selected_category != ShowcaseCategory::None
                                                && tile.category != ui_state.selected_category
                                                && matches!(tile.action, TileAction::Launch(_));
                                            let (rect, response) =
                                                ui.allocate_exact_size(tile_size, egui::Sense::click());
                                            paint_glass_tile(
                                                ui.painter(), rect, tile, response.hovered(), dimmed,
                                            );
                                            if response.clicked() {
                                                match tile.action {
                                                    TileAction::Launch(module) => {
                                                        cmd = LauncherCommand::OpenModule(module);
                                                    }
                                                    TileAction::Help => {
                                                        ui_state.show_help = !ui_state.show_help;
                                                    }
                                                    TileAction::SubMenu => {
                                                        ui_state.show_submenu = !ui_state.show_submenu;
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            });
                        });
                    });
                    ui.add_space(16.0);
                });
        });

    if ui_state.show_submenu {
        let mut open = ui_state.show_submenu;
        egui::Window::new(crate::i18n::text("nav.008"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-26.0, -48.0))
            .show(ctx, |ui| {
                if ui.button(crate::i18n::text("nav.009")).clicked() {
                    ui_state.detailed_view = true;
                    ui_state.show_submenu = false;
                }
                ui.separator();
                if ui.button(crate::i18n::text("nav.010")).clicked() {
                    cmd = LauncherCommand::OpenModule("aqua");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.011")).clicked() {
                    cmd = LauncherCommand::OpenModule("sim");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.012")).clicked() {
                    cmd = LauncherCommand::OpenModule("lca");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.013")).clicked() {
                    cmd = LauncherCommand::OpenModule("mass_balance");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.014")).clicked() {
                    cmd = LauncherCommand::OpenModule("excel_addins");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.015")).clicked() {
                    cmd = LauncherCommand::OpenModule("excel_addins");
                    ui_state.show_submenu = false;
                }
                if ui.button(crate::i18n::text("nav.016")).clicked() {
                    cmd = LauncherCommand::OpenModule("species");
                    ui_state.show_submenu = false;
                }
                ui.separator();
                if ui.button(crate::i18n::text("nav.017")).clicked() {
                    ui_state.selected_category = ShowcaseCategory::None;
                    ui_state.show_submenu = false;
                }
            });
        ui_state.show_submenu = open;
    }

    if ui_state.show_settings {
        let mut open = ui_state.show_settings;
        egui::Window::new(crate::i18n::text("nav.018"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(crate::i18n::text("nav.019")).strong().size(13.0));
                ui.checkbox(&mut ui_state.detailed_view, crate::i18n::text("nav.020"));
                ui.separator();
                ui.label(format!("Especies cargadas: {species_count}"));
                ui.label(crate::i18n::text("nav.021"));
                ui.label(crate::i18n::text("nav.022"));
            });
        ui_state.show_settings = open;
    }

    if ui_state.show_help {
        let mut open = ui_state.show_help;
        egui::Window::new(crate::i18n::text("nav.023"))
            .open(&mut open)
            .default_size([460.0, 290.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(crate::i18n::text("nav.024")).strong().size(14.0));
                ui.separator();
                ui.label(crate::i18n::text("nav.025"));
                ui.label(crate::i18n::text("nav.026"));
                ui.label(crate::i18n::text("nav.027"));
                ui.label(crate::i18n::text("nav.028"));
            });
        ui_state.show_help = open;
    }

    cmd
}

fn toggle_category(ui_state: &mut LauncherUiState, category: ShowcaseCategory) {
    ui_state.selected_category = if ui_state.selected_category == category {
        ShowcaseCategory::None
    } else {
        category
    };
}

// ---------------------------------------------------------------------------
// Painter Renderers for Fig. 1 Industrial Scene, Left Cards & Frosted Tiles
// ---------------------------------------------------------------------------

fn paint_gear_button(p: &egui::Painter, rect: egui::Rect, hovered: bool) {
    let c = rect.center();
    let bg = if hovered {
        egui::Color32::from_rgb(220, 232, 248)
    } else {
        egui::Color32::from_rgb(226, 232, 240)
    };
    p.circle_filled(c, 14.0, bg);
    p.circle_stroke(c, 14.0, egui::Stroke::new(1.2, egui::Color32::from_rgb(148, 163, 184)));
    let col = egui::Color32::from_rgb(71, 85, 105);
    p.circle_stroke(c, 7.2, egui::Stroke::new(2.8, col));
    p.circle_stroke(c, 3.0, egui::Stroke::new(1.4, col));
    for k in 0..8 {
        let ang = (k as f32) * std::f32::consts::FRAC_PI_4;
        let dir = egui::vec2(ang.cos(), ang.sin());
        p.line_segment([c + dir * 7.5, c + dir * 10.6], egui::Stroke::new(2.2, col));
    }
}

fn paint_industrial_backdrop(p: &egui::Painter, rect: egui::Rect) {
    // Deep blue sky bands + steel refinery horizon silhouettes matching Fig. 1
    let bands = [
        (0.00, 0.25, egui::Color32::from_rgb(9, 32, 66)),
        (0.25, 0.50, egui::Color32::from_rgb(17, 55, 104)),
        (0.50, 0.72, egui::Color32::from_rgb(24, 72, 128)),
        (0.72, 1.00, egui::Color32::from_rgb(18, 30, 48)),
    ];
    for (t0, t1, col) in bands {
        let b = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.min.y + rect.height() * t0),
            egui::pos2(rect.max.x, rect.min.y + rect.height() * t1),
        );
        p.rect_filled(b, 0.0, col);
    }

    // Subtle atmospheric horizon glow behind the center grid
    p.circle_filled(
        egui::pos2(rect.center().x, rect.min.y + rect.height() * 0.56),
        rect.width() * 0.42,
        egui::Color32::from_rgba_unmultiplied(120, 185, 245, 12),
    );

    // Subtle metallurgical towers / pipe silhouettes at lower horizon
    let sil_col = egui::Color32::from_rgba_unmultiplied(10, 18, 30, 82);
    let hi_col = egui::Color32::from_rgba_unmultiplied(165, 200, 235, 32);
    let base_y = rect.max.y;
    let specs: [(f32, f32, f32); 11] = [
        (0.06, 0.08, 0.42),
        (0.16, 0.06, 0.52),
        (0.27, 0.11, 0.46),
        (0.41, 0.05, 0.64),
        (0.48, 0.13, 0.48),
        (0.62, 0.07, 0.56),
        (0.71, 0.10, 0.50),
        (0.81, 0.04, 0.68),
        (0.86, 0.08, 0.44),
        (0.93, 0.07, 0.50),
        (0.00, 1.00, 0.16),
    ];
    for (x0, w, h) in specs {
        let r = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + rect.width() * x0, base_y - rect.height() * h),
            egui::pos2(rect.min.x + rect.width() * (x0 + w), base_y),
        );
        p.rect_filled(r, 1.0, sil_col);
        p.line_segment([r.left_top(), r.right_top()], egui::Stroke::new(1.0, hi_col));
    }
}

fn paint_showcase_card(
    p: &egui::Painter,
    rect: egui::Rect,
    cat: ShowcaseCategory,
    title: &str,
    active: bool,
    hovered: bool,
) {
    // Mini scenic backgrounds resembling Fig. 1 left column artwork
    match cat {
        ShowcaseCategory::Products => {
            p.rect_filled(rect, 2.0, egui::Color32::from_rgb(10, 24, 52));
            let horizon = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.center().y - 6.0),
                rect.max,
            );
            p.rect_filled(horizon, 0.0, egui::Color32::from_rgb(38, 126, 206));
            p.line_segment(
                [horizon.left_top(), horizon.right_top()],
                egui::Stroke::new(3.0, egui::Color32::from_rgb(165, 225, 255)),
            );
        }
        ShowcaseCategory::Services => {
            p.rect_filled(rect, 2.0, egui::Color32::from_rgb(176, 198, 222));
            let lower = egui::Rect::from_min_max(rect.left_center(), rect.max);
            p.rect_filled(lower, 0.0, egui::Color32::from_rgb(122, 148, 178));
            for dx in [0.18, 0.45, 0.72] {
                let x = rect.min.x + rect.width() * dx;
                p.rect_filled(
                    egui::Rect::from_min_max(egui::pos2(x, rect.min.y + 24.0), egui::pos2(x + 22.0, rect.max.y)),
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(235, 244, 255, 130),
                );
            }
        }
        ShowcaseCategory::Sustainability => {
            p.rect_filled(rect, 2.0, egui::Color32::from_rgb(20, 126, 165));
            let forest = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + 34.0),
                egui::pos2(rect.max.x, rect.min.y + 66.0),
            );
            p.rect_filled(forest, 0.0, egui::Color32::from_rgb(38, 148, 62));
            let water = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y + 66.0),
                rect.max,
            );
            p.rect_filled(water, 0.0, egui::Color32::from_rgb(18, 92, 142));
        }
        ShowcaseCategory::ResearchDev | ShowcaseCategory::None => {
            p.rect_filled(rect, 2.0, egui::Color32::from_rgb(36, 36, 40));
            let ember = egui::Rect::from_min_max(
                egui::pos2(rect.min.x + 18.0, rect.max.y - 24.0),
                egui::pos2(rect.max.x - 18.0, rect.max.y - 2.0),
            );
            p.rect_filled(ember, 6.0, egui::Color32::from_rgb(225, 62, 20));
            p.circle_filled(ember.center(), 24.0, egui::Color32::from_rgba_unmultiplied(255, 115, 30, 100));
        }
    }

    // Subtle dark header strip for readable title
    let s = (rect.height() / 104.0).clamp(0.68, 1.12);
    let label_bg = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + 24.0 * s));
    p.rect_filled(label_bg, 0.0, egui::Color32::from_rgba_unmultiplied(8, 14, 24, 95));

    let title_col = if cat == ShowcaseCategory::Services {
        egui::Color32::from_rgb(15, 23, 42)
    } else {
        egui::Color32::WHITE
    };
    p.text(
        rect.left_top() + egui::vec2(9.0 * s, 7.5 * s),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(13.0 * s),
        title_col,
    );

    let border = if active {
        egui::Stroke::new(2.2, egui::Color32::from_rgb(56, 189, 248))
    } else if hovered {
        egui::Stroke::new(1.5, egui::Color32::WHITE)
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 85))
    };
    p.rect_stroke(rect, 2.0, border, egui::StrokeKind::Middle);
}

fn paint_glass_tile(
    p: &egui::Painter,
    rect: egui::Rect,
    tile: &TileSpec,
    hovered: bool,
    dimmed: bool,
) {
    let fill = if dimmed {
        egui::Color32::from_rgba_unmultiplied(17, 34, 55, 185)
    } else if hovered {
        egui::Color32::from_rgb(43, 86, 128)
    } else {
        egui::Color32::from_rgba_unmultiplied(27, 57, 88, 240)
    };
    p.rect_filled(rect, 4.0, fill);
    let border_alpha = if tile.running {
        220
    } else if hovered {
        180
    } else if dimmed {
        35
    } else {
        105
    };
    let border_col = if tile.running {
        egui::Color32::from_rgba_unmultiplied(74, 222, 128, border_alpha)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, border_alpha)
    };
    p.rect_stroke(rect, 2.0, egui::Stroke::new(if tile.running { 1.6 } else { 1.0 }, border_col), egui::StrokeKind::Middle);

    let text_alpha = if dimmed { 180 } else { 255 };
    let main_col = egui::Color32::from_rgba_unmultiplied(255, 255, 255, text_alpha);
    let sub_col = egui::Color32::from_rgba_unmultiplied(230, 238, 248, text_alpha);

    let s = (rect.height() / 102.0).clamp(0.68, 1.12);
    if matches!(tile.action, TileAction::SubMenu) {
        // Layered SUB MENU graphic matching Fig. 1 bottom-right tile
        let r1 = egui::Rect::from_min_size(rect.left_top() + egui::vec2(14.0 * s, 16.0 * s), egui::vec2(15.0 * s, 15.0 * s));
        let r2 = egui::Rect::from_min_size(rect.left_top() + egui::vec2(20.0 * s, 21.0 * s), egui::vec2(15.0 * s, 15.0 * s));
        p.rect_stroke(r1, 3.0, egui::Stroke::new(1.8, main_col), egui::StrokeKind::Middle);
        p.rect_stroke(r2, 3.0, egui::Stroke::new(1.8, main_col), egui::StrokeKind::Middle);
        p.text(
            rect.left_bottom() + egui::vec2(13.0 * s, -14.0 * s),
            egui::Align2::LEFT_BOTTOM,
            tile.label,
            egui::FontId::proportional(14.5 * s),
            main_col,
        );
    } else if tile.code == "?" {
        p.text(
            rect.right_top() + egui::vec2(-18.0 * s, 14.0 * s),
            egui::Align2::RIGHT_TOP,
            "?",
            egui::FontId::proportional(28.0 * s),
            main_col,
        );
        p.text(
            rect.left_bottom() + egui::vec2(12.0 * s, -13.0 * s),
            egui::Align2::LEFT_BOTTOM,
            tile.label,
            egui::FontId::proportional(11.0 * s),
            sub_col,
        );
    } else {
        p.text(
            rect.left_top() + egui::vec2(12.0 * s, 12.0 * s),
            egui::Align2::LEFT_TOP,
            tile.code,
            egui::FontId::proportional(26.0 * s),
            main_col,
        );
        p.text(
            rect.left_bottom() + egui::vec2(12.0 * s, -12.0 * s),
            egui::Align2::LEFT_BOTTOM,
            tile.label,
            egui::FontId::proportional(11.0 * s),
            sub_col,
        );
    }

    // Green active-window indicator dot in top-right corner
    if tile.running {
        p.circle_filled(
            rect.right_top() + egui::vec2(-10.0 * s, 10.0 * s),
            4.5 * s,
            egui::Color32::from_rgb(74, 222, 128),
        );
    }
}
