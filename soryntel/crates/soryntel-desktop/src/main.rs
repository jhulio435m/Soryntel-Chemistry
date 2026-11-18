//! Soryntel Chemistry — Native Desktop Application (Módulos 01, 02 & 03).
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

rust_i18n::i18n!("locales", fallback = "es");

mod i18n;
mod storage;
#[cfg(test)]
mod test_support;
mod addins_bridge;
mod aqua_ui;
mod module_link;
mod balance_ui;
mod benson_ui;
mod composition;
mod diagrams_ui;
mod ehph_ui;
mod elements_ui;
mod equilibrium_ui;
mod estimate_ui;
mod excel_addins_ui;
mod excel_grid;
mod exergy_ui;
mod heat_loss_ui;
mod launcher_ui;
mod lca_ui;
mod mass_balance_ui;
mod phase_stability_ui;
mod reaction_actions;
mod reaction_ui;
mod sim_ui;
mod species_converter_ui;
mod species_ui;
mod ui_common;
mod units_ui;
mod water_ui;

use composition::AppServices;
use eframe::egui;
use soryntel_application::use_cases::{AddinsUseCases, CatalogUseCases};
#[cfg(test)]
use soryntel_db::ThermoDatabase;

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowMode {
    Launcher,
    Reaction,
    Balance,
    HeatLoss,
    Equilibrium,
    Exergy,
    Estimate,
    EhPh,
    Diagrams,
    PhaseStability,
    SpeciesConverter,
    Benson,
    Elements,
    Water,
    Units,
    ExcelAddIns,
    Aqua,
    Sim,
    Lca,
    MassBalance,
    Species,
}

fn spawn_module_window(mode_arg: &str, custom_db: Option<&str>) -> Option<std::process::Child> {
    let exe = std::env::current_exe().ok()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--module").arg(mode_arg);
    if let Some(p) = custom_db {
        cmd.arg("--hsc-db").arg(p);
    }
    cmd.spawn().ok()
}

fn is_child_running(slot: &mut Option<std::process::Child>) -> bool {
    if let Some(child) = slot.as_mut() {
        match child.try_wait() {
            Ok(None) => return true,
            _ => {
                *slot = None;
            }
        }
    }
    false
}

/// Envía (si procede) una pérdida de calor de Heat Loss a Balance y devuelve el mensaje para
/// el usuario. Si el Balance abierto no tiene el área elegida, no se envía nada.
fn heat_loss_send_outcome(
    send: impl FnOnce() -> Result<(), String>,
    open_balance_areas: Option<usize>,
    target_area: usize,
) -> (String, bool) {
    if let Some(n) = open_balance_areas {
        if target_area >= n {
            return (
                format!(
                    "El Balance abierto tiene {n} área(s); el Area {} no existe. Elija otra área.",
                    target_area + 1
                ),
                false,
            );
        }
    }
    if let Err(e) = send() {
        return (format!("No se pudo enviar la pérdida de calor a Balance: {e}"), false);
    }
    match open_balance_areas {
        Some(_) => (
            format!("Enviado al Balance abierto (Area {}); esperando confirmación…", target_area + 1),
            true,
        ),
        None => (
            format!(
                "No había ningún Balance abierto: se abre uno con el libro por defecto y la pérdida se importará en su Area {}.",
                target_area + 1
            ),
            true,
        ),
    }
}

/// Lanza un módulo en su ranura solo si no hay ya un proceso vivo en ella: así nunca se
/// pierde el `Child` de una ventana abierta (que «Cerrar todo» no podría cerrar).
/// Devuelve `true` si se lanzó un proceso nuevo.
fn launch_into_slot(
    slot: &mut Option<std::process::Child>,
    spawn: impl FnOnce() -> Option<std::process::Child>,
) -> bool {
    if is_child_running(slot) {
        return false;
    }
    *slot = spawn();
    slot.is_some()
}

fn kill_child(slot: &mut Option<std::process::Child>) {
    if let Some(mut child) = slot.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// Módulos aceptados por `--module` (y sus alias).
const MODULE_ARGS: &[(&str, WindowMode)] = &[
    ("launcher", WindowMode::Launcher),
    ("reaction", WindowMode::Reaction),
    ("balance", WindowMode::Balance),
    ("heatloss", WindowMode::HeatLoss),
    ("equilibrium", WindowMode::Equilibrium),
    ("exergy", WindowMode::Exergy),
    ("estimate", WindowMode::Estimate),
    ("ehph", WindowMode::EhPh),
    ("diagrams", WindowMode::Diagrams),
    ("phase_stability", WindowMode::PhaseStability),
    ("species_converter", WindowMode::SpeciesConverter),
    ("benson", WindowMode::Benson),
    ("elements", WindowMode::Elements),
    ("water", WindowMode::Water),
    ("units", WindowMode::Units),
    ("excel_addins", WindowMode::ExcelAddIns),
    ("addins", WindowMode::ExcelAddIns),
    ("geo", WindowMode::ExcelAddIns),
    ("aqua", WindowMode::Aqua),
    ("sim", WindowMode::Sim),
    ("flowsheet", WindowMode::Sim),
    ("lca", WindowMode::Lca),
    ("sim_lca", WindowMode::Lca),
    ("mass_balance", WindowMode::MassBalance),
    ("mas", WindowMode::MassBalance),
    ("species", WindowMode::Species),
    ("databases", WindowMode::Species),
    ("browser", WindowMode::Species),
];

struct CliOptions {
    custom_db: Option<String>,
    window_mode: WindowMode,
    bridge_port: Option<u16>,
    eval_cli: Option<Vec<String>>,
}

/// Interpreta los argumentos (sin el nombre del programa). Un `--module` desconocido o un
/// `--hsc-db`/`--module` sin valor es un error, en lugar de abrir el launcher en silencio.
fn parse_cli_args(args: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions {
        custom_db: None,
        window_mode: WindowMode::Launcher,
        bridge_port: None,
        eval_cli: None,
    };
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--hsc-db" {
            let value = args.get(i + 1).filter(|v| !v.starts_with("--"));
            let Some(value) = value else {
                return Err("--hsc-db necesita la ruta de MainDB".into());
            };
            options.custom_db = Some(value.clone());
            i += 2;
        } else if let Some(val) = a.strip_prefix("--hsc-db=") {
            if val.is_empty() {
                return Err("--hsc-db necesita la ruta de MainDB".into());
            }
            options.custom_db = Some(val.to_string());
            i += 1;
        } else if a == "--addin-bridge" {
            let port = args.get(i + 1).and_then(|s| s.parse::<u16>().ok());
            options.bridge_port = Some(port.unwrap_or(8765));
            i += if port.is_some() { 2 } else { 1 };
        } else if let Some(val) = a.strip_prefix("--addin-bridge=") {
            options.bridge_port = Some(val.parse::<u16>().unwrap_or(8765));
            i += 1;
        } else if a == "--addin-eval" && i + 1 < args.len() {
            options.eval_cli = Some(args[i + 1..].to_vec());
            break;
        } else if a == "--module" || a.starts_with("--module=") {
            let (value, step) = match a.strip_prefix("--module=") {
                Some(v) => (Some(v), 1),
                None => (args.get(i + 1).map(String::as_str), 2),
            };
            let Some(value) = value.filter(|v| !v.is_empty()) else {
                return Err("--module necesita el nombre de un módulo".into());
            };
            options.window_mode = MODULE_ARGS
                .iter()
                .find(|(name, _)| *name == value)
                .map(|(_, mode)| *mode)
                .ok_or_else(|| {
                    let valid: Vec<&str> = MODULE_ARGS.iter().map(|(name, _)| *name).collect();
                    format!("módulo desconocido «{value}». Valores válidos: {}", valid.join(", "))
                })?;
            i += step;
        } else {
            i += 1;
        }
    }
    Ok(options)
}

fn main() -> eframe::Result<()> {
    rust_i18n::set_locale("es");

    let args: Vec<String> = std::env::args().collect();
    let CliOptions { custom_db, window_mode, bridge_port, eval_cli } = match parse_cli_args(&args[1..]) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("soryntel-desktop: {message}");
            std::process::exit(2);
        }
    };
    let services = AppServices::load(custom_db.as_deref());

    if let Some(port) = bridge_port {
        let db_arc = services.shared_catalog();
        let units_arc = std::sync::Arc::new(std::sync::Mutex::new(
            soryntel_application::models::addins::AddInUnits::default(),
        ));
        if let Err(err) = addins_bridge::run_bridge_blocking(db_arc, units_arc, port) {
            eprintln!("Error iniciando HTTP Bridge en puerto {port}: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    if let Some(cli_args) = eval_cli {
        let mut units = soryntel_application::models::addins::AddInUnits::default();
        let target = &cli_args[0];
        let val = if target.trim_start().starts_with('=') {
            AddinsUseCases::eval_addin_formula(&*services.catalog(), &mut units, target, |_, _| {
                String::new()
            })
        } else {
            let fn_upper = target.trim().to_ascii_uppercase();
            let args_vec: Vec<soryntel_application::models::addins::ArgValue> = cli_args[1..]
                .iter()
                .enumerate()
                .map(|(idx, s)| {
                    if s.contains('|')
                        || ((fn_upper.starts_with("STREAM")
                            || fn_upper.starts_with("FLOW")
                            || fn_upper == "DENSITYA"
                            || matches!(fn_upper.as_str(), "SUM" | "AVERAGE" | "MIN" | "MAX"))
                            && idx <= 1
                            && (s.contains(';') || s.contains(',')))
                    {
                        let sep = if s.contains('|') {
                            '|'
                        } else if s.contains(';') {
                            ';'
                        } else {
                            ','
                        };
                        soryntel_application::models::addins::ArgValue::Range(
                            s.split(sep).map(|x| x.trim().to_string()).collect(),
                        )
                    } else {
                        soryntel_application::models::addins::ArgValue::Scalar(s.clone())
                    }
                })
                .collect();
            AddinsUseCases::eval_addin_call(&*services.catalog(), &mut units, target, &args_vec)
        };
        println!("{}", val.as_multiline_string(8));
        return Ok(());
    }

    let (title, size, min_size) = match window_mode {
        WindowMode::Launcher => (
            crate::i18n::text("window.000"),
            [1120.0, 680.0],
            [720.0, 480.0],
        ),
        WindowMode::Reaction => (
            crate::i18n::text("window.001"),
            [1340.0, 820.0],
            [480.0, 360.0],
        ),
        WindowMode::Balance => (
            crate::i18n::text("window.002"),
            [1380.0, 840.0],
            [480.0, 360.0],
        ),
        WindowMode::HeatLoss => (
            crate::i18n::text("window.003"),
            [1340.0, 820.0],
            [480.0, 360.0],
        ),
        WindowMode::Equilibrium => (
            crate::i18n::text("window.004"),
            [1400.0, 860.0],
            [480.0, 360.0],
        ),
        WindowMode::Exergy => (
            crate::i18n::text("window.005"),
            [1380.0, 820.0],
            [480.0, 360.0],
        ),
        WindowMode::Estimate => (
            crate::i18n::text("window.006"),
            [1100.0, 700.0],
            [480.0, 360.0],
        ),
        WindowMode::EhPh => (
            crate::i18n::text("window.007"),
            [1180.0, 740.0],
            [480.0, 360.0],
        ),
        WindowMode::Diagrams => (
            crate::i18n::text("window.008"),
            [1260.0, 760.0],
            [480.0, 360.0],
        ),
        WindowMode::PhaseStability => (
            crate::i18n::text("window.009"),
            [1220.0, 740.0],
            [480.0, 360.0],
        ),
        WindowMode::SpeciesConverter => (
            crate::i18n::text("window.010"),
            [1280.0, 760.0],
            [480.0, 360.0],
        ),
        WindowMode::Benson => (
            crate::i18n::text("window.011"),
            [1220.0, 760.0],
            [480.0, 360.0],
        ),
        WindowMode::Elements => (
            crate::i18n::text("window.012"),
            [1320.0, 780.0],
            [480.0, 360.0],
        ),
        WindowMode::Water => (
            crate::i18n::text("window.013"),
            [990.0, 630.0],
            [480.0, 360.0],
        ),
        WindowMode::Units => (
            crate::i18n::text("window.014"),
            [720.0, 660.0],
            [480.0, 360.0],
        ),
        WindowMode::ExcelAddIns => (
            crate::i18n::text("window.015"),
            [1220.0, 760.0],
            [480.0, 360.0],
        ),
        WindowMode::Aqua => ("Aqua — Soryntel Chemistry", [1320.0, 780.0], [720.0, 480.0]),
        WindowMode::Sim => (
            "HSC Sim — Soryntel Chemistry",
            [1380.0, 820.0],
            [820.0, 520.0],
        ),
        WindowMode::Lca => (
            crate::i18n::text("window.016"),
            [1360.0, 820.0],
            [860.0, 520.0],
        ),
        WindowMode::MassBalance => (
            crate::i18n::text("window.017"),
            [1360.0, 820.0],
            [920.0, 560.0],
        ),
        WindowMode::Species => (
            crate::i18n::text("window.018"),
            [1220.0, 760.0],
            [640.0, 480.0],
        ),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size(min_size)
            .with_title(title),
        ..Default::default()
    };

    let custom_db_clone = custom_db.clone();
    eframe::run_native(
        title,
        options,
        Box::new(move |cc| {
            configure_soryntel_theme(&cc.egui_ctx);
            Ok(Box::new(SoryntelDesktopApp::new(
                services,
                window_mode,
                custom_db_clone,
            )))
        }),
    )
}

fn configure_soryntel_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = egui::Color32::from_rgb(244, 246, 249);
    visuals.window_fill = egui::Color32::from_rgb(250, 251, 253);
    visuals.extreme_bg_color = egui::Color32::WHITE;
    visuals.faint_bg_color = egui::Color32::from_rgb(238, 241, 246);
    visuals.selection.bg_fill = egui::Color32::from_rgb(29, 78, 216);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(214, 219, 226));
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(203, 210, 220));
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(239, 246, 255);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(59, 130, 246));
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(219, 234, 254);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    visuals.window_rounding = egui::Rounding::same(6.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.button_padding = egui::vec2(9.0, 4.5);
    style.spacing.window_margin = egui::Margin::same(10.0);
    ctx.set_style(style);
}

#[derive(Clone, Copy)]
struct ModuleRunningFlags {
    reaction: bool,
    balance: bool,
    heat_loss: bool,
    equilibrium: bool,
    exergy: bool,
    estimate: bool,
    ehph: bool,
    diagrams: bool,
    phase_stability: bool,
    species_converter: bool,
    benson: bool,
    elements: bool,
    water: bool,
    units: bool,
    excel_addins: bool,
    aqua: bool,
    sim: bool,
    lca: bool,
    mass_balance: bool,
    browser: bool,
}

impl ModuleRunningFlags {
    fn is_running(&self, mode_arg: &str) -> bool {
        match mode_arg {
            "reaction" => self.reaction,
            "balance" => self.balance,
            "heatloss" => self.heat_loss,
            "equilibrium" => self.equilibrium,
            "exergy" => self.exergy,
            "estimate" => self.estimate,
            "ehph" => self.ehph,
            "diagrams" => self.diagrams,
            "phase_stability" => self.phase_stability,
            "species_converter" => self.species_converter,
            "benson" => self.benson,
            "elements" => self.elements,
            "water" => self.water,
            "units" => self.units,
            "excel_addins" | "addins" | "geo" => self.excel_addins,
            "aqua" => self.aqua,
            "sim" | "flowsheet" => self.sim,
            "lca" | "sim_lca" => self.lca,
            "mass_balance" | "mas" => self.mass_balance,
            "species" | "databases" | "browser" => self.browser,
            _ => false,
        }
    }
}

struct SoryntelDesktopApp {
    services: AppServices,
    window_mode: WindowMode,
    custom_db_arg: Option<String>,
    child_reaction: Option<std::process::Child>,
    child_balance: Option<std::process::Child>,
    child_heat_loss: Option<std::process::Child>,
    child_equilibrium: Option<std::process::Child>,
    child_exergy: Option<std::process::Child>,
    child_estimate: Option<std::process::Child>,
    child_ehph: Option<std::process::Child>,
    child_diagrams: Option<std::process::Child>,
    child_phase_stability: Option<std::process::Child>,
    child_species_converter: Option<std::process::Child>,
    child_benson: Option<std::process::Child>,
    child_elements: Option<std::process::Child>,
    child_water: Option<std::process::Child>,
    child_units: Option<std::process::Child>,
    child_excel_addins: Option<std::process::Child>,
    child_aqua: Option<std::process::Child>,
    child_sim: Option<std::process::Child>,
    child_lca: Option<std::process::Child>,
    child_mass_balance: Option<std::process::Child>,
    child_browser: Option<std::process::Child>,
    reaction_state: reaction_ui::ReactionModuleState,
    aqua_state: aqua_ui::AquaModuleState,
    balance_state: balance_ui::BalanceModuleState,
    heat_loss_state: heat_loss_ui::HeatLossModuleState,
    equilibrium_state: equilibrium_ui::EquilibriumUiState,
    exergy_state: exergy_ui::ExergyModuleState,
    estimate_state: estimate_ui::EstimateModuleState,
    ehph_state: ehph_ui::EhPhModuleState,
    diagrams_state: diagrams_ui::DiagramsModuleState,
    phase_stability_state: phase_stability_ui::PhaseStabilityModuleState,
    species_converter_state: species_converter_ui::SpeciesConverterModuleState,
    benson_state: benson_ui::BensonModuleState,
    elements_state: elements_ui::ElementsModuleState,
    water_state: water_ui::WaterModuleState,
    units_state: units_ui::UnitsModuleState,
    excel_addins_state: excel_addins_ui::ExcelAddInsModuleState,
    sim_state: sim_ui::SimModuleState,
    lca_standalone_project: soryntel_application::models::simulation::SimProject,
    lca_state: lca_ui::LcaWindowState,
    mb_standalone_project: soryntel_application::models::simulation::SimProject,
    mb_state: mass_balance_ui::MassBalanceWindowState,
    species_state: species_ui::SpeciesModuleState,
    launcher_ui_state: launcher_ui::LauncherUiState,
    /// Último número de áreas publicado para Heat Loss (solo en la ventana Balance).
    published_balance_areas: Option<usize>,
    db_warnings_dismissed: bool,
    last_formula_poll: Option<std::time::Instant>,
}

impl SoryntelDesktopApp {
    fn new(services: AppServices, window_mode: WindowMode, custom_db_arg: Option<String>) -> Self {
        // Un único guard de lectura para construir los estados; se suelta antes de mover
        // `services` a `Self`. `&dyn SpeciesWorkspacePort` se convierte por upcasting en el
        // puerto que pida cada módulo.
        let guard = services.catalog();
        let db = &*guard;
        let reaction_state = reaction_ui::ReactionModuleState::new(db);
        let aqua_state = aqua_ui::AquaModuleState::new(db);
        let balance_state = balance_ui::BalanceModuleState::new(db);
        let heat_loss_state = heat_loss_ui::HeatLossModuleState::new();
        let equilibrium_state = equilibrium_ui::EquilibriumUiState::default();
        let exergy_state = exergy_ui::ExergyModuleState::new(db);
        let estimate_state = estimate_ui::EstimateModuleState::new(db);
        let ehph_state = ehph_ui::EhPhModuleState::new(db);
        let diagrams_state = diagrams_ui::DiagramsModuleState::new(db);
        let phase_stability_state = phase_stability_ui::PhaseStabilityModuleState::new(db);
        let species_converter_state =
            species_converter_ui::SpeciesConverterModuleState::new(db);
        let benson_state = benson_ui::BensonModuleState::new(db);
        let elements_state = elements_ui::ElementsModuleState::default();
        let water_state = water_ui::WaterModuleState::new(db);
        let units_state = units_ui::UnitsModuleState::default();
        let excel_addins_state = excel_addins_ui::ExcelAddInsModuleState::default();
        let sim_state = sim_ui::SimModuleState::default();
        let (lca_standalone_project, lca_state) = lca_ui::LcaWindowState::with_preset_classic_zn();
        let (mb_standalone_project, mb_state) =
            mass_balance_ui::MassBalanceWindowState::with_preset_rougher();
        let species_state = species_ui::SpeciesModuleState::new(db);
        drop(guard);
        Self {
            services,
            window_mode,
            custom_db_arg,
            child_reaction: None,
            child_balance: None,
            child_heat_loss: None,
            child_equilibrium: None,
            child_exergy: None,
            child_estimate: None,
            child_ehph: None,
            child_diagrams: None,
            child_phase_stability: None,
            child_species_converter: None,
            child_benson: None,
            child_elements: None,
            child_water: None,
            child_units: None,
            child_excel_addins: None,
            child_aqua: None,
            child_sim: None,
            child_lca: None,
            child_mass_balance: None,
            child_browser: None,
            reaction_state,
            aqua_state,
            balance_state,
            heat_loss_state,
            equilibrium_state,
            exergy_state,
            estimate_state,
            ehph_state,
            diagrams_state,
            phase_stability_state,
            species_converter_state,
            benson_state,
            elements_state,
            water_state,
            units_state,
            excel_addins_state,
            sim_state,
            lca_standalone_project,
            lca_state,
            mb_standalone_project,
            mb_state,
            species_state,
            launcher_ui_state: launcher_ui::LauncherUiState::default(),
            published_balance_areas: None,
            db_warnings_dismissed: false,
            last_formula_poll: None,
        }
    }
}

impl eframe::App for SoryntelDesktopApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.window_mode == WindowMode::Balance {
            module_link::ModuleLink::in_temp_dir().clear_presence();
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_database_warnings(ctx);
        match self.window_mode {
            WindowMode::Reaction => {
                // Fórmulas enviadas desde Species: sondeo cada 500 ms, no en cada frame.
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
                let due = self
                    .last_formula_poll
                    .is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(500));
                if due {
                    self.last_formula_poll = Some(std::time::Instant::now());
                    if let Some(formula) = module_link::ModuleLink::in_temp_dir().take_formula() {
                        self.reaction_state.formula_input = formula;
                        self.reaction_state.run_calculate(&*self.services.catalog());
                    }
                }
                self.reaction_state.ui(ctx, &*self.services.catalog());
                if self.reaction_state.request_open_browser {
                    self.reaction_state.request_open_browser = false;
                    if !is_child_running(&mut self.child_browser) {
                        self.child_browser =
                            spawn_module_window("species", self.custom_db_arg.as_deref());
                    }
                }
                return;
            }
            WindowMode::Balance => {
                // Sondeo periódico del envío de Heat Loss aunque no haya eventos de entrada.
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
                let link = module_link::ModuleLink::in_temp_dir();
                if let Some(received) = link.take_heat_loss() {
                    match received {
                        Ok((area, q_w)) => self.services.with_catalog(|catalog| {
                            self.balance_state.set_area_heat_loss_w(area, q_w, catalog)
                        }),
                        Err(e) => {
                            self.balance_state.status_msg = e;
                            self.balance_state.status_ok = false;
                        }
                    }
                    let _ = link.write_ack(self.balance_state.status_ok, &self.balance_state.status_msg);
                }
                self.services
                    .with_catalog(|catalog| self.balance_state.ui(ctx, catalog));
                let areas = self.balance_state.wb.areas.len();
                if self.published_balance_areas != Some(areas) && link.publish_presence(areas).is_ok() {
                    self.published_balance_areas = Some(areas);
                }
                return;
            }
            WindowMode::HeatLoss => {
                ctx.request_repaint_after(std::time::Duration::from_millis(500));
                let link = module_link::ModuleLink::in_temp_dir();
                // Áreas del Balance abierto (otro proceso); 0 si no hay ninguno.
                let open_balance_areas = link.balance_area_count();
                self.heat_loss_state.ui(ctx, open_balance_areas.unwrap_or(0));
                if let Some(q_w) = self.heat_loss_state.pending_export_w.take() {
                    let target_area = self.heat_loss_state.export_target_area;
                    let (msg, ok) = heat_loss_send_outcome(
                        || link.send_heat_loss(target_area, q_w).map_err(|e| e.to_string()),
                        open_balance_areas,
                        target_area,
                    );
                    if ok && open_balance_areas.is_none() && !is_child_running(&mut self.child_balance) {
                        self.child_balance =
                            spawn_module_window("balance", self.custom_db_arg.as_deref());
                    }
                    self.heat_loss_state.status_msg = msg;
                    self.heat_loss_state.status_ok = ok;
                }
                if let Some((ok, msg)) = link.take_ack() {
                    self.heat_loss_state.status_msg = format!("Balance: {msg}");
                    self.heat_loss_state.status_ok = ok;
                }
                return;
            }
            WindowMode::Equilibrium => {
                self.equilibrium_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::Exergy => {
                self.exergy_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::Estimate => {
                // Guard de escritura temporal: se suelta al terminar la sentencia.
                self.estimate_state.ui(ctx, &mut *self.services.catalog_mut());
                return;
            }
            WindowMode::EhPh => {
                self.ehph_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::Diagrams => {
                self.diagrams_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::PhaseStability => {
                self.phase_stability_state
                    .ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::SpeciesConverter => {
                self.services
                    .with_catalog(|catalog| self.species_converter_state.ui(ctx, catalog));
                return;
            }
            WindowMode::Benson => {
                self.benson_state.ui(ctx, &mut *self.services.catalog_mut());
                return;
            }
            WindowMode::Elements => {
                self.services
                    .with_catalog(|catalog| self.elements_state.ui(ctx, catalog));
                return;
            }
            WindowMode::Water => {
                self.water_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::Units => {
                self.units_state.ui(ctx);
                return;
            }
            WindowMode::ExcelAddIns => {
                self.excel_addins_state.ui(ctx, self.services.shared_catalog());
                return;
            }
            WindowMode::Aqua => {
                self.aqua_state.ui(ctx, &*self.services.catalog());
                return;
            }
            WindowMode::Sim => {
                self.sim_state.ui(ctx);
                return;
            }
            WindowMode::Lca => {
                lca_ui::render_standalone_lca_module(
                    ctx,
                    &mut self.lca_standalone_project,
                    &mut self.lca_state,
                );
                return;
            }
            WindowMode::MassBalance => {
                mass_balance_ui::render_standalone_mb_module(
                    ctx,
                    &mut self.mb_standalone_project,
                    &mut self.mb_state,
                );
                return;
            }
            WindowMode::Species => {
                // El guard de escritura se suelta al terminar esta sentencia, antes de
                // tomar el de lectura para recalcular la reacción.
                let selected = self.species_state.ui(ctx, &mut *self.services.catalog_mut());
                if let Some(f) = selected {
                    match module_link::ModuleLink::in_temp_dir().send_formula(&f) {
                        Ok(()) => {
                            self.species_state.status_msg = format!(
                                "«{f}» enviada a Reaction: se cargará si hay una ventana de Reaction abierta en los próximos {} s.",
                                module_link::FORMULA_MAX_AGE_SECS
                            );
                            self.species_state.status_is_err = false;
                        }
                        Err(e) => {
                            self.species_state.status_msg = format!("No se pudo enviar «{f}» a Reaction: {e}");
                            self.species_state.status_is_err = true;
                        }
                    }
                }
                return;
            }
            WindowMode::Launcher => {}
        }

        self.show_launcher_ui(ctx);
    }
}

impl SoryntelDesktopApp {
    /// Banner con los fallos de carga de MainDB/OwnDB (en Windows release no hay consola).
    fn show_database_warnings(&mut self, ctx: &egui::Context) {
        if self.db_warnings_dismissed || self.services.load_warnings().is_empty() {
            return;
        }
        egui::TopBottomPanel::top("db_load_warnings")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(254, 243, 199))
                    .inner_margin(egui::Margin::symmetric(10.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("⚠ Bases de datos").strong().color(egui::Color32::from_rgb(146, 64, 14)));
                    for warning in self.services.load_warnings() {
                        ui.label(egui::RichText::new(warning).color(egui::Color32::from_rgb(120, 53, 15)));
                    }
                    if ui.small_button("Cerrar aviso").clicked() {
                        self.db_warnings_dismissed = true;
                    }
                });
            });
    }

    fn show_launcher_ui(&mut self, ctx: &egui::Context) {
        // Poll child processes periodically while launcher is open
        ctx.request_repaint_after(std::time::Duration::from_millis(400));
        let running_reaction = is_child_running(&mut self.child_reaction);
        let running_balance = is_child_running(&mut self.child_balance);
        let running_heat_loss = is_child_running(&mut self.child_heat_loss);
        let running_equilibrium = is_child_running(&mut self.child_equilibrium);
        let running_exergy = is_child_running(&mut self.child_exergy);
        let running_estimate = is_child_running(&mut self.child_estimate);
        let running_ehph = is_child_running(&mut self.child_ehph);
        let running_diagrams = is_child_running(&mut self.child_diagrams);
        let running_phase_stability = is_child_running(&mut self.child_phase_stability);
        let running_species_converter = is_child_running(&mut self.child_species_converter);
        let running_benson = is_child_running(&mut self.child_benson);
        let running_elements = is_child_running(&mut self.child_elements);
        let running_water = is_child_running(&mut self.child_water);
        let running_units = is_child_running(&mut self.child_units);
        let running_excel_addins = is_child_running(&mut self.child_excel_addins);
        let running_aqua = is_child_running(&mut self.child_aqua);
        let running_sim = is_child_running(&mut self.child_sim);
        let running_lca = is_child_running(&mut self.child_lca);
        let running_mass_balance = is_child_running(&mut self.child_mass_balance);
        let running_browser = is_child_running(&mut self.child_browser);

        let open_count = [
            running_reaction,
            running_balance,
            running_heat_loss,
            running_equilibrium,
            running_exergy,
            running_estimate,
            running_ehph,
            running_diagrams,
            running_phase_stability,
            running_species_converter,
            running_benson,
            running_elements,
            running_water,
            running_units,
            running_excel_addins,
            running_aqua,
            running_sim,
            running_lca,
            running_mass_balance,
            running_browser,
        ]
        .into_iter()
        .filter(|&b| b)
        .count();

        launcher_ui::render_hsc10_header(
            ctx,
            &mut self.launcher_ui_state,
            self.services.with_catalog(|catalog| CatalogUseCases::species_count(catalog)),
        );
        let footer_cmd =
            launcher_ui::render_hsc10_footer(ctx, &mut self.launcher_ui_state, open_count);
        match footer_cmd {
            launcher_ui::LauncherCommand::KillAll => self.kill_all_children(),
            launcher_ui::LauncherCommand::OpenModule(m) => self.launch_module_by_key(m),
            launcher_ui::LauncherCommand::None => {}
        }

        if !self.launcher_ui_state.detailed_view {
            use launcher_ui::{ShowcaseCategory as Cat, TileAction as Act, TileSpec};
            let tiles: [TileSpec; 24] = [
                // Row 1 (Fig. 1 top row)
                TileSpec { code: "Aqu", label: crate::i18n::text("nav.029"), action: Act::Launch("aqua"), category: Cat::Sustainability, running: running_aqua },
                TileSpec { code: "Bal", label: crate::i18n::text("nav.030"), action: Act::Launch("balance"), category: Cat::Products, running: running_balance },
                TileSpec { code: "EpH", label: crate::i18n::text("nav.031"), action: Act::Launch("ehph"), category: Cat::Sustainability, running: running_ehph },
                TileSpec { code: "Est", label: crate::i18n::text("nav.032"), action: Act::Launch("estimate"), category: Cat::ResearchDev, running: running_estimate },
                TileSpec { code: "Mea", label: crate::i18n::text("nav.033"), action: Act::Launch("units"), category: Cat::Services, running: running_units },
                TileSpec { code: "Nav", label: crate::i18n::text("nav.034"), action: Act::Launch("diagrams"), category: Cat::Products, running: running_diagrams },
                // Row 2
                TileSpec { code: "Ben", label: crate::i18n::text("nav.035"), action: Act::Launch("benson"), category: Cat::ResearchDev, running: running_benson },
                TileSpec { code: "Con", label: crate::i18n::text("nav.036"), action: Act::Launch("species_converter"), category: Cat::Services, running: running_species_converter },
                TileSpec { code: "Exe", label: crate::i18n::text("nav.037"), action: Act::Launch("exergy"), category: Cat::Sustainability, running: running_exergy },
                TileSpec { code: "Gem", label: crate::i18n::text("nav.038"), action: Act::Launch("equilibrium"), category: Cat::Products, running: running_equilibrium },
                TileSpec { code: "Rea", label: crate::i18n::text("nav.039"), action: Act::Launch("reaction"), category: Cat::Products, running: running_reaction },
                TileSpec { code: "Sim", label: crate::i18n::text("nav.040"), action: Act::Launch("sim"), category: Cat::Products, running: running_sim },
                // Row 3
                TileSpec { code: "Dat", label: crate::i18n::text("nav.041"), action: Act::Launch("excel_addins"), category: Cat::Services, running: running_excel_addins },
                TileSpec { code: "DB", label: crate::i18n::text("window.019"), action: Act::Launch("species"), category: Cat::ResearchDev, running: running_browser },
                TileSpec { code: "Geo", label: crate::i18n::text("nav.042"), action: Act::Launch("excel_addins"), category: Cat::Services, running: running_excel_addins },
                TileSpec { code: "HTr", label: crate::i18n::text("nav.043"), action: Act::Launch("heatloss"), category: Cat::Services, running: running_heat_loss },
                TileSpec { code: "Tpp", label: crate::i18n::text("nav.044"), action: Act::Launch("phase_stability"), category: Cat::ResearchDev, running: running_phase_stability },
                TileSpec { code: "Wat", label: crate::i18n::text("nav.045"), action: Act::Launch("water"), category: Cat::Sustainability, running: running_water },
                // Row 4
                TileSpec { code: "Dia", label: crate::i18n::text("nav.046"), action: Act::Launch("diagrams"), category: Cat::ResearchDev, running: running_diagrams },
                TileSpec { code: "Ele", label: crate::i18n::text("nav.047"), action: Act::Launch("elements"), category: Cat::ResearchDev, running: running_elements },
                TileSpec { code: "Mas", label: crate::i18n::text("nav.048"), action: Act::Launch("mass_balance"), category: Cat::Products, running: running_mass_balance },
                TileSpec { code: "Lca", label: crate::i18n::text("nav.049"), action: Act::Launch("lca"), category: Cat::Sustainability, running: running_lca },
                TileSpec { code: "?", label: crate::i18n::text("nav.050"), action: Act::Help, category: Cat::None, running: false },
                TileSpec { code: "SUB", label: crate::i18n::text("window.020"), action: Act::SubMenu, category: Cat::None, running: false },
            ];
            match launcher_ui::render_hsc10_grid_canvas(
                ctx,
                &mut self.launcher_ui_state,
                &tiles,
                self.services.with_catalog(|catalog| CatalogUseCases::species_count(catalog)),
            ) {
                launcher_ui::LauncherCommand::OpenModule(m) => self.launch_module_by_key(m),
                launcher_ui::LauncherCommand::KillAll => self.kill_all_children(),
                launcher_ui::LauncherCommand::None => {}
            }
            return;
        }

        // Classic Detailed Cards View (toggled via SUB MENU or footer pill)
        let status_flags = ModuleRunningFlags {
            reaction: running_reaction,
            balance: running_balance,
            heat_loss: running_heat_loss,
            equilibrium: running_equilibrium,
            exergy: running_exergy,
            estimate: running_estimate,
            ehph: running_ehph,
            diagrams: running_diagrams,
            phase_stability: running_phase_stability,
            species_converter: running_species_converter,
            benson: running_benson,
            elements: running_elements,
            water: running_water,
            units: running_units,
            excel_addins: running_excel_addins,
            aqua: running_aqua,
            sim: running_sim,
            lca: running_lca,
            mass_balance: running_mass_balance,
            browser: running_browser,
        };
        self.show_cards_view(ctx, status_flags);
    }

    fn show_cards_view(&mut self, ctx: &egui::Context, flags: ModuleRunningFlags) {
        use launcher_ui::ShowcaseCategory as Cat;

        struct ModuleCardSpec {
            num: &'static str,
            code: &'static str,
            title: &'static str,
            desc: &'static str,
            accent: egui::Color32,
            mode_arg: &'static str,
            category: Cat,
            is_new: bool,
        }

        let cards: [ModuleCardSpec; 21] = [
            ModuleCardSpec {
                num: "01",
                code: "Rea",
                title: crate::i18n::text("nav.051"),
                desc: "Cálculos termodinámicos de especies y reacciones: Cp, H, S, G, K y potenciales electroquímicos.",
                accent: egui::Color32::from_rgb(37, 99, 235),
                mode_arg: "reaction",
                category: Cat::Products,
                is_new: false,
            },
            ModuleCardSpec {
                num: "02",
                code: "Bal",
                title: crate::i18n::text("nav.052"),
                desc: "Balances de masa, energía y exergía con flujos, áreas de proceso y hojas tipo Excel.",
                accent: egui::Color32::from_rgb(13, 148, 136),
                mode_arg: "balance",
                category: Cat::Products,
                is_new: false,
            },
            ModuleCardSpec {
                num: "03",
                code: "HTr",
                title: crate::i18n::text("nav.053"),
                desc: "Transferencia de calor multicapa por conducción, convección y radiación con exportación a Balance.",
                accent: egui::Color32::from_rgb(217, 119, 6),
                mode_arg: "heatloss",
                category: Cat::Services,
                is_new: false,
            },
            ModuleCardSpec {
                num: "04",
                code: "Gem",
                title: crate::i18n::text("nav.054"),
                desc: "Minimización de energía libre de Gibbs multifase (GEM), barridos 1D/2D y gráficos 2D/3D.",
                accent: egui::Color32::from_rgb(124, 58, 237),
                mode_arg: "equilibrium",
                category: Cat::Products,
                is_new: false,
            },
            ModuleCardSpec {
                num: "05",
                code: "Exe",
                title: crate::i18n::text("nav.055"),
                desc: "Balances de exergía química (Szargut 1988), física, mezcla ideal y calor por Carnot.",
                accent: egui::Color32::from_rgb(21, 128, 61),
                mode_arg: "exergy",
                category: Cat::Sustainability,
                is_new: false,
            },
            ModuleCardSpec {
                num: "06",
                code: "Est",
                title: crate::i18n::text("window.021"),
                desc: "Estimación de H, S, Cp, ΔG (Neumann-Kopp, Latimer, Benson), paridad vs Maindb6 y ajuste Kelley.",
                accent: egui::Color32::from_rgb(14, 116, 144),
                mode_arg: "estimate",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "07",
                code: "EpH",
                title: crate::i18n::text("nav.056"),
                desc: "Diagramas de Pourbaix (STABCAL), límites del agua, Criss-Cobble, Davies y Combiner multi-dataset.",
                accent: egui::Color32::from_rgb(124, 58, 237),
                mode_arg: "ehph",
                category: Cat::Sustainability,
                is_new: false,
            },
            ModuleCardSpec {
                num: "08",
                code: "Dia/Nav",
                title: "H, S, C and G Diagrams",
                desc: "Diagramas termoquímicos (H total/latent, S, Cp, G, ΔH, ΔS), Navigator y diagramas de Ellingham.",
                accent: egui::Color32::from_rgb(180, 83, 9),
                mode_arg: "diagrams",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "09",
                code: "Tpp",
                title: crate::i18n::text("nav.057"),
                desc: "Diagramas de predominancia T-p (Tpp) y p-p isotérmicos (Lpp) para sistemas ternarios y Table Values.",
                accent: egui::Color32::from_rgb(79, 70, 229),
                mode_arg: "phase_stability",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "10",
                code: "Con",
                title: crate::i18n::text("nav.058"),
                desc: "Conversión mineralógica elementos ↔ especies (NNLS Métodos 1-3), Element Balance, Fix Balance y Charts.",
                accent: egui::Color32::from_rgb(190, 24, 93),
                mode_arg: "species_converter",
                category: Cat::Services,
                is_new: false,
            },
            ModuleCardSpec {
                num: "11",
                code: "Ben",
                title: crate::i18n::text("nav.059"),
                desc: "Constructor 3D molecular interactivo y estimación orgánica por grupos de Benson (GAV) G/L/S con guardado DB.",
                accent: egui::Color32::from_rgb(29, 78, 216),
                mode_arg: "benson",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "12",
                code: "Ele",
                title: crate::i18n::text("nav.060"),
                desc: "Tabla Periódica interactiva (Z=1..103), esferas proporcionales por propiedad, Element4 editable y Maindb6.",
                accent: egui::Color32::from_rgb(5, 150, 105),
                mode_arg: "elements",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "13",
                code: "Wat",
                title: crate::i18n::text("nav.061"),
                desc: "Tablas de vapor interactivas p-T (-270..2000 °C, 0.001..1000 bar), saturación, Mixture % y exergía Szargut.",
                accent: egui::Color32::from_rgb(2, 132, 199),
                mode_arg: "water",
                category: Cat::Sustainability,
                is_new: false,
            },
            ModuleCardSpec {
                num: "14",
                code: "Mea",
                title: crate::i18n::text("window.022"),
                desc: "Conversor bidireccional (90 magnitudes, 444 unidades + prefijos SI), CODATA, Tamices Mesh y Humedad.",
                accent: egui::Color32::from_rgb(217, 119, 6),
                mode_arg: "units",
                category: Cat::Services,
                is_new: false,
            },
            ModuleCardSpec {
                num: "15",
                code: "Dat",
                title: crate::i18n::text("window.023"),
                desc: "83 funciones HSC6/HSC10 + Mineralogía, StreamEQ matricial, exportador .xlsx/.ods/ONLYOFFICE y puente HTTP.",
                accent: egui::Color32::from_rgb(33, 115, 70),
                mode_arg: "excel_addins",
                category: Cat::Services,
                is_new: false,
            },
            ModuleCardSpec {
                num: "16",
                code: "DB",
                title: crate::i18n::text("window.024"),
                desc: "Búsqueda estequiométrica y orgánica (OwnDB > Maindb6), Editor OwnDB, Fit Cp, Convert Cp y G(T) -> H,S,Cp.",
                accent: egui::Color32::from_rgb(51, 65, 85),
                mode_arg: "species",
                category: Cat::ResearchDev,
                is_new: false,
            },
            ModuleCardSpec {
                num: "17",
                code: "Aqu",
                title: crate::i18n::text("window.025"),
                desc: "Electrolitos Pitzer / Harvie / Davies, Active Parameter Sets, OwnDB editable, Density (59 sales) y Plot AC vs T.",
                accent: egui::Color32::from_rgb(2, 132, 199),
                mode_arg: "aqua",
                category: Cat::Sustainability,
                is_new: true,
            },
            ModuleCardSpec {
                num: "18",
                code: "Sim",
                title: crate::i18n::text("window.026"),
                desc: "Editor de diagramas de flujo, Stream Tables, Scenario Editor (Monte Carlo), Control Loops, Tank, Tarjan y Reports.",
                accent: egui::Color32::from_rgb(16, 108, 180),
                mode_arg: "sim",
                category: Cat::Products,
                is_new: true,
            },
            ModuleCardSpec {
                num: "19",
                code: "Lca",
                title: crate::i18n::text("window.027"),
                desc: "LCA Evaluation (GaBi EcoSpold v1.0, OpenLCA JSON-LD .zip, Exergy, *,?,#) + Beta IPC/LCIA (ecoinvent 3.8, ReCiPe).",
                accent: egui::Color32::from_rgb(22, 163, 74),
                mode_arg: "lca",
                category: Cat::Sustainability,
                is_new: true,
            },
            ModuleCardSpec {
                num: "20",
                code: "Mas",
                title: crate::i18n::text("window.028"),
                desc: "Reconciliación EWTLS+WLS por flujos, agua/%sólidos y PSD (Sized/Assay), LS/MinMax/NNLS/CLS/LLS y Export Excel.",
                accent: egui::Color32::from_rgb(234, 88, 12),
                mode_arg: "mass_balance",
                category: Cat::Products,
                is_new: true,
            },
            ModuleCardSpec {
                num: "GEO",
                code: "Geo",
                title: "Mineral Database (HSC Geo)",
                desc: "Catálogo de 3,812 minerales y funciones MineralCode/Name/Formula/Density integradas con Excel Add-Ins y Mas.",
                accent: egui::Color32::from_rgb(101, 163, 13),
                mode_arg: "excel_addins",
                category: Cat::Services,
                is_new: false,
            },
        ];

        let mut pending_action: Option<(&'static str, bool)> = None;
        let species_count = self.services.with_catalog(|catalog| CatalogUseCases::species_count(catalog));
        let search_q = self.launcher_ui_state.cards_search.trim().to_lowercase();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(244, 246, 249))
                    .inner_margin(egui::Margin::symmetric(16.0, 12.0)),
            )
            .show(ctx, |ui| {
                // Header + Filters Bar
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("MÓDULOS")
                            .strong()
                            .size(14.5)
                            .color(egui::Color32::from_rgb(15, 23, 42)),
                    );
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(219, 234, 254))
                        .rounding(10.0)
                        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("20 módulos + Geo")
                                    .strong()
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(30, 64, 175)),
                            );
                        });
                    ui.label(
                        egui::RichText::new(
                            "Cada módulo se abre en una ventana independiente (paridad HSC 10 Grid)",
                        )
                        .size(11.5)
                        .color(egui::Color32::from_rgb(100, 116, 139)),
                    );
                });
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Buscar:")
                            .size(11.5)
                            .color(egui::Color32::from_rgb(71, 85, 105)),
                    );
                    ui.add_sized(
                        [200.0, 24.0],
                        egui::TextEdit::singleline(&mut self.launcher_ui_state.cards_search)
                            .hint_text("Sim, Lca, Mas, Aqu, DB..."),
                    );
                    if !self.launcher_ui_state.cards_search.is_empty()
                        && ui.small_button("✖").clicked()
                    {
                        self.launcher_ui_state.cards_search.clear();
                    }
                    ui.separator();

                    let all_active = self.launcher_ui_state.cards_category == Cat::None
                        && !self.launcher_ui_state.cards_recent_only;
                    if ui.selectable_label(all_active, "Todos (21)").clicked() {
                        self.launcher_ui_state.cards_category = Cat::None;
                        self.launcher_ui_state.cards_recent_only = false;
                    }
                    if ui
                        .selectable_label(
                            self.launcher_ui_state.cards_recent_only,
                            "Recientes · 17–20 (4)",
                        )
                        .clicked()
                    {
                        self.launcher_ui_state.cards_recent_only =
                            !self.launcher_ui_state.cards_recent_only;
                    }
                    for (cat, lbl) in [
                        (Cat::Products, crate::i18n::text("nav.004")),
                        (Cat::Services, crate::i18n::text("nav.005")),
                        (Cat::Sustainability, crate::i18n::text("nav.006")),
                        (Cat::ResearchDev, "Research & Dev"),
                    ] {
                        let sel = self.launcher_ui_state.cards_category == cat;
                        if ui.selectable_label(sel, lbl).clicked() {
                            self.launcher_ui_state.cards_category =
                                if sel { Cat::None } else { cat };
                        }
                    }
                });
                ui.add_space(8.0);

                let filtered: Vec<&ModuleCardSpec> = cards
                    .iter()
                    .filter(|c| {
                        if self.launcher_ui_state.cards_recent_only && !c.is_new {
                            return false;
                        }
                        if self.launcher_ui_state.cards_category != Cat::None
                            && c.category != self.launcher_ui_state.cards_category
                        {
                            return false;
                        }
                        if search_q.is_empty() {
                            return true;
                        }
                        c.num.to_lowercase().contains(&search_q)
                            || c.code.to_lowercase().contains(&search_q)
                            || c.title.to_lowercase().contains(&search_q)
                            || c.desc.to_lowercase().contains(&search_q)
                            || c.mode_arg.to_lowercase().contains(&search_q)
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .id_salt("classic_cards_scroll")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.add_space(24.0);
                            ui.label(
                                egui::RichText::new(
                                    "No hay módulos que coincidan con el filtro actual.",
                                )
                                .size(13.0)
                                .color(egui::Color32::from_rgb(100, 116, 139)),
                            );
                        } else {
                            let avail_w = ui.available_width();
                            let ncols = if avail_w >= 900.0 {
                                3
                            } else if avail_w >= 560.0 {
                                2
                            } else {
                                1
                            };

                            for (row_idx, chunk) in filtered.chunks(ncols).enumerate() {
                                if row_idx > 0 {
                                    ui.add_space(10.0);
                                }
                                let draw_card =
                                    |ui: &mut egui::Ui,
                                     card: &ModuleCardSpec,
                                     pending: &mut Option<(&'static str, bool)>| {
                                        let is_open = flags.is_running(card.mode_arg);
                                        let border_color = if is_open {
                                            card.accent
                                        } else {
                                            egui::Color32::from_rgb(214, 219, 226)
                                        };
                                        egui::Frame::none()
                                            .fill(egui::Color32::WHITE)
                                            .stroke(egui::Stroke::new(
                                                if is_open { 1.6 } else { 1.0 },
                                                border_color,
                                            ))
                                            .rounding(8.0)
                                            .inner_margin(egui::Margin::same(11.0))
                                            .show(ui, |ui| {
                                                ui.set_min_height(132.0);
                                                ui.horizontal(|ui| {
                                                    let badge_lbl = if card.num == "GEO" {
                                                        "HERR. GEO".to_string()
                                                    } else {
                                                        format!("MÓDULO {}", card.num)
                                                    };
                                                    egui::Frame::none()
                                                        .fill(card.accent)
                                                        .rounding(5.0)
                                                        .inner_margin(egui::Margin::symmetric(
                                                            7.0, 2.5,
                                                        ))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(badge_lbl)
                                                                    .strong()
                                                                    .size(10.0)
                                                                    .color(egui::Color32::WHITE),
                                                            );
                                                        });
                                                    egui::Frame::none()
                                                        .fill(egui::Color32::from_rgb(241, 245, 249))
                                                        .rounding(4.0)
                                                        .inner_margin(egui::Margin::symmetric(
                                                            6.0, 2.0,
                                                        ))
                                                        .show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(card.code)
                                                                    .strong()
                                                                    .size(10.0)
                                                                    .color(egui::Color32::from_rgb(
                                                                        51, 65, 85,
                                                                    )),
                                                            );
                                                        });
                                                    if card.is_new {
                                                        egui::Frame::none()
                                                            .fill(egui::Color32::from_rgb(
                                                                254, 243, 199,
                                                            ))
                                                            .rounding(10.0)
                                                            .inner_margin(egui::Margin::symmetric(
                                                                6.0, 2.0,
                                                            ))
                                                            .show(ui, |ui| {
                                                                ui.label(
                                                                    egui::RichText::new("NUEVO")
                                                                        .strong()
                                                                        .size(9.5)
                                                                        .color(
                                                                            egui::Color32::from_rgb(
                                                                                180, 83, 9,
                                                                            ),
                                                                        ),
                                                                );
                                                            });
                                                    }
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if is_open {
                                                                egui::Frame::none()
                                                                    .fill(egui::Color32::from_rgb(
                                                                        220, 252, 231,
                                                                    ))
                                                                    .rounding(10.0)
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            7.0, 2.0,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        ui.label(
                                                                            egui::RichText::new(
                                                                                "● Abierto",
                                                                            )
                                                                            .strong()
                                                                            .size(10.0)
                                                                            .color(
                                                                                egui::Color32::from_rgb(
                                                                                    22, 101, 52,
                                                                                ),
                                                                            ),
                                                                        );
                                                                    });
                                                            }
                                                        },
                                                    );
                                                });
                                                ui.add_space(5.0);
                                                ui.label(
                                                    egui::RichText::new(card.title)
                                                        .strong()
                                                        .size(14.0)
                                                        .color(egui::Color32::from_rgb(15, 23, 42)),
                                                );
                                                ui.add_space(3.0);
                                                let desc_txt = if card.mode_arg == "species" {
                                                    format!(
                                                        "{species_count} especies (OwnDB > Maindb6): Búsqueda estequiométrica/orgánica, Editor OwnDB, Fit/Convert Cp y G(T)."
                                                    )
                                                } else {
                                                    card.desc.to_string()
                                                };
                                                ui.label(
                                                    egui::RichText::new(desc_txt)
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            71, 85, 105,
                                                        )),
                                                );
                                                ui.add_space(8.0);
                                                ui.horizontal(|ui| {
                                                    let primary_lbl = if is_open {
                                                        "Abrir otra ventana ↗"
                                                    } else {
                                                        "Abrir módulo ↗"
                                                    };
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                egui::RichText::new(primary_lbl)
                                                                    .strong()
                                                                    .size(11.5)
                                                                    .color(egui::Color32::WHITE),
                                                            )
                                                            .fill(card.accent)
                                                            .rounding(5.0),
                                                        )
                                                        .clicked()
                                                    {
                                                        *pending = Some((card.mode_arg, false));
                                                    }
                                                    if is_open
                                                        && ui
                                                            .add(
                                                                egui::Button::new(
                                                                    egui::RichText::new("Cerrar")
                                                                        .size(11.5)
                                                                        .color(
                                                                            egui::Color32::from_rgb(
                                                                                71, 85, 105,
                                                                            ),
                                                                        ),
                                                                )
                                                                .fill(egui::Color32::from_rgb(
                                                                    241, 245, 249,
                                                                ))
                                                                .rounding(5.0),
                                                            )
                                                            .clicked()
                                                    {
                                                        *pending = Some((card.mode_arg, true));
                                                    }
                                                });
                                            });
                                    };

                                if ncols == 1 {
                                    draw_card(ui, chunk[0], &mut pending_action);
                                } else {
                                    ui.columns(ncols, |cols| {
                                        for (idx, card) in chunk.iter().enumerate() {
                                            draw_card(&mut cols[idx], card, &mut pending_action);
                                        }
                                    });
                                }
                            }
                        }

                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new("HERRAMIENTAS Y BASE DE DATOS")
                                .strong()
                                .size(12.0)
                                .color(egui::Color32::from_rgb(71, 85, 105)),
                        );
                        ui.add_space(5.0);
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(214, 219, 226),
                            ))
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(
                                                crate::i18n::text("window.029"),
                                            )
                                            .strong()
                                            .size(13.0)
                                            .color(egui::Color32::from_rgb(15, 23, 42)),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{species_count} especies activas (OwnDB > Maindb6) · 3,812 minerales HSC Geo · 83 funciones Excel.",
                                            ))
                                            .size(11.2)
                                            .color(egui::Color32::from_rgb(100, 116, 139)),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let lbl = if flags.browser {
                                                "Cerrar Species DB"
                                            } else {
                                                "Abrir Species DB ↗"
                                            };
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new(lbl)
                                                            .strong()
                                                            .size(11.5)
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(egui::Color32::from_rgb(51, 65, 85))
                                                    .rounding(5.0),
                                                )
                                                .clicked()
                                            {
                                                pending_action = Some(("species", flags.browser));
                                            }
                                        },
                                    );
                                });
                            });
                        ui.add_space(8.0);
                    });
            });

        if let Some((mode_arg, should_kill)) = pending_action {
            let custom = self.custom_db_arg.clone();
            if let Some(slot) = self.module_slot_mut(mode_arg) {
                if should_kill {
                    kill_child(slot);
                } else {
                    launch_into_slot(slot, || spawn_module_window(mode_arg, custom.as_deref()));
                }
            }
        }
    }

    fn module_slot_mut(&mut self, mode_arg: &str) -> Option<&mut Option<std::process::Child>> {
        match mode_arg {
            "reaction" => Some(&mut self.child_reaction),
            "balance" => Some(&mut self.child_balance),
            "heatloss" => Some(&mut self.child_heat_loss),
            "equilibrium" => Some(&mut self.child_equilibrium),
            "exergy" => Some(&mut self.child_exergy),
            "estimate" => Some(&mut self.child_estimate),
            "ehph" => Some(&mut self.child_ehph),
            "diagrams" => Some(&mut self.child_diagrams),
            "phase_stability" => Some(&mut self.child_phase_stability),
            "species_converter" => Some(&mut self.child_species_converter),
            "benson" => Some(&mut self.child_benson),
            "elements" => Some(&mut self.child_elements),
            "water" => Some(&mut self.child_water),
            "units" => Some(&mut self.child_units),
            "excel_addins" | "addins" | "geo" => Some(&mut self.child_excel_addins),
            "aqua" => Some(&mut self.child_aqua),
            "sim" | "flowsheet" => Some(&mut self.child_sim),
            "lca" | "sim_lca" => Some(&mut self.child_lca),
            "mass_balance" | "mas" => Some(&mut self.child_mass_balance),
            "species" | "databases" | "browser" => Some(&mut self.child_browser),
            _ => None,
        }
    }

    fn launch_module_by_key(&mut self, mode_arg: &str) {
        let custom = self.custom_db_arg.clone();
        if let Some(slot) = self.module_slot_mut(mode_arg) {
            launch_into_slot(slot, || spawn_module_window(mode_arg, custom.as_deref()));
        }
    }

    fn kill_all_children(&mut self) {
        for slot in [
            &mut self.child_reaction,
            &mut self.child_balance,
            &mut self.child_heat_loss,
            &mut self.child_equilibrium,
            &mut self.child_exergy,
            &mut self.child_estimate,
            &mut self.child_ehph,
            &mut self.child_diagrams,
            &mut self.child_phase_stability,
            &mut self.child_species_converter,
            &mut self.child_benson,
            &mut self.child_elements,
            &mut self.child_water,
            &mut self.child_units,
            &mut self.child_excel_addins,
            &mut self.child_aqua,
            &mut self.child_sim,
            &mut self.child_lca,
            &mut self.child_mass_balance,
            &mut self.child_browser,
        ] {
            kill_child(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn launching_a_running_module_keeps_the_tracked_child() {
        let spawn = || std::process::Command::new("sleep").arg("30").spawn().ok();
        let mut slot = None;
        assert!(launch_into_slot(&mut slot, spawn));
        let first = slot.as_ref().unwrap().id();
        let mut spawned_again = false;
        assert!(!launch_into_slot(&mut slot, || {
            spawned_again = true;
            spawn()
        }));
        assert!(!spawned_again, "no se lanza un segundo proceso del mismo módulo");
        assert_eq!(slot.as_ref().unwrap().id(), first);
        kill_child(&mut slot);
        assert!(slot.is_none());
        // Tras cerrarse, el módulo puede volver a abrirse.
        assert!(launch_into_slot(&mut slot, spawn));
        kill_child(&mut slot);
    }

    #[test]
    fn cli_rejects_unknown_module_and_missing_values() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let ok = parse_cli_args(&args(&["--module", "geo", "--hsc-db", "/db ñ/Maindb6.HSC"])).unwrap();
        assert!(ok.window_mode == WindowMode::ExcelAddIns);
        assert_eq!(ok.custom_db.as_deref(), Some("/db ñ/Maindb6.HSC"));
        assert!(parse_cli_args(&args(&["--module=balance"])).unwrap().window_mode == WindowMode::Balance);
        let err = parse_cli_args(&args(&["--module", "equilibirum"])).err().unwrap();
        assert!(err.contains("equilibirum") && err.contains("equilibrium"), "{err}");
        assert!(parse_cli_args(&args(&["--module"])).is_err());
        assert!(parse_cli_args(&args(&["--hsc-db"])).is_err());
        assert!(parse_cli_args(&args(&["--hsc-db", "--module", "reaction"])).is_err());
        assert!(parse_cli_args(&args(&[])).unwrap().window_mode == WindowMode::Launcher);
        // Cada módulo que el launcher puede abrir se acepta en --module.
        for m in ["reaction", "balance", "heatloss", "equilibrium", "exergy", "estimate", "ehph", "diagrams",
                  "phase_stability", "species_converter", "benson", "elements", "water", "units",
                  "excel_addins", "aqua", "sim", "lca", "mass_balance", "species"] {
            assert!(parse_cli_args(&args(&["--module", m])).is_ok(), "{m}");
        }
    }

    #[test]
    fn heat_loss_send_checks_target_area_before_sending() {
        let mut sent = false;
        let (msg, ok) = heat_loss_send_outcome(|| { sent = true; Ok(()) }, Some(1), 2);
        assert!(!ok && !sent && msg.contains("Area 3"), "{msg}");
        let (_, ok) = heat_loss_send_outcome(|| Ok(()), Some(3), 2);
        assert!(ok);
        let (msg, ok) = heat_loss_send_outcome(|| Err("disco lleno".into()), None, 0);
        assert!(!ok && msg.contains("disco lleno"));
        let (msg, ok) = heat_loss_send_outcome(|| Ok(()), None, 0);
        assert!(ok && msg.contains("No había ningún Balance"));
    }

    fn collect_rendered_texts(
        app: &mut SoryntelDesktopApp,
        size: [f32; 2],
        scroll_passes: usize,
    ) -> Vec<String> {
        let ctx = egui::Context::default();
        configure_soryntel_theme(&ctx);
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(size[0], size[1]),
            )),
            ..Default::default()
        };
        let mut texts = Vec::new();
        let total = scroll_passes + 2;
        for pass in 0..total {
            let mut input = raw_input.clone();
            if pass >= 1 && pass <= scroll_passes {
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -300.0),
                    modifiers: egui::Modifiers::NONE,
                });
                input
                    .events
                    .push(egui::Event::PointerMoved(egui::pos2(500.0, 300.0)));
            }
            let full_output = ctx.run(input, |ctx| {
                app.show_launcher_ui(ctx);
            });
            if pass == total - 1 {
                fn visit(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
                    match shape {
                        egui::epaint::Shape::Vec(v) => {
                            for s in v {
                                visit(s, out);
                            }
                        }
                        egui::epaint::Shape::Text(ts) => {
                            out.push(ts.galley.text().to_string());
                        }
                        _ => {}
                    }
                }
                for clipped in &full_output.shapes {
                    visit(&clipped.shape, &mut texts);
                }
            }
        }
        texts
    }

    #[test]
    fn cards_view_renders_all_20_modules_and_geo() {
        let db = ThermoDatabase::with_open_defaults();
        let mut app =
            SoryntelDesktopApp::new(AppServices::from_catalog(db), WindowMode::Launcher, None);
        app.launcher_ui_state.detailed_view = true;
        let texts = collect_rendered_texts(&mut app, [1120.0, 2200.0], 0);
        for num in 1..=20 {
            let badge = format!("MÓDULO {num:02}");
            assert!(
                texts.iter().any(|t| t == &badge),
                "Missing card {badge} in Classic Cards View"
            );
        }
        assert!(texts.iter().any(|t| t == "HERR. GEO"));
        assert!(texts
            .iter()
            .any(|t| t.contains(crate::i18n::text("window.028"))));
        assert!(texts
            .iter()
            .any(|t| t.contains(crate::i18n::text("window.027"))));
        assert!(texts
            .iter()
            .any(|t| t.contains(crate::i18n::text("window.026"))));
        assert!(texts.iter().any(|t| t.contains(crate::i18n::text("window.025"))));
    }

    #[test]
    fn cards_view_recent_filter_and_scroll_show_latest_modules() {
        let db = ThermoDatabase::with_open_defaults();
        let mut app =
            SoryntelDesktopApp::new(AppServices::from_catalog(db), WindowMode::Launcher, None);
        app.launcher_ui_state.detailed_view = true;

        // With Recientes 17-20 filter, all 4 newest modules appear immediately in 1120x680
        app.launcher_ui_state.cards_recent_only = true;
        let recent_texts = collect_rendered_texts(&mut app, [1120.0, 680.0], 0);
        for badge in ["MÓDULO 17", "MÓDULO 18", "MÓDULO 19", "MÓDULO 20"] {
            assert!(
                recent_texts.iter().any(|t| t == badge),
                "Expected {badge} under Recientes 17-20 filter"
            );
        }
        assert!(!recent_texts.iter().any(|t| t == "MÓDULO 01"));

        // With all 21 cards shown, scrolling in default 1120x680 reveals 19, 20, Geo
        app.launcher_ui_state.cards_recent_only = false;
        let scrolled_texts = collect_rendered_texts(&mut app, [1120.0, 680.0], 4);
        assert!(scrolled_texts.iter().any(|t| t == "MÓDULO 19"));
        assert!(scrolled_texts.iter().any(|t| t == "MÓDULO 20"));
        assert!(scrolled_texts.iter().any(|t| t == "HERR. GEO"));
    }
}
