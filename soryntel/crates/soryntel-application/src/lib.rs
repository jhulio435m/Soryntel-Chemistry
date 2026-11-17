//! Hexagonal Application Layer (Inbound Use-Case Ports).
//!
//! Depends strictly on domain crates (`soryntel-core`, `soryntel-solver`, `soryntel-thermo`)
//! and outbound port traits (`SpeciesRepository`, `SpeciesCatalogPort`) — never on concrete
//! database or UI adapters.

use crate::ports::SpeciesCatalogPort;
use soryntel_core::SoryntelError;
use soryntel_thermo::{
    aqua::{
        analyze_parameter_sets as domain_aqua_analyze_parameter_sets,
        calculate_aqua as domain_aqua_calculate, calculate_solution_density as domain_aqua_density,
        resolve_pitzer_catalog as domain_aqua_resolve_catalog, ActiveParameterSetsReport,
        AquaCalcResult, AquaCaseFile, DensityCalcResult, DensityInputRow, MergedPitzerCatalog,
        OwnPitzerDb,
    },
    aqua_pitzer_data::{builtin_main_pitzer_db as domain_aqua_main_pitzer_db, PitzerDatabase},
    are_responses_plot_compatible as domain_reaction_plot_compatible,
    balance::estimate_air_cooled_reactor_heat_loss as domain_estimate_air_cooled_heat_loss,
    balance_engine::{
        build_balance_export_sheets as domain_build_balance_export_sheets,
        build_linked_paste_stream as domain_build_linked_paste_stream,
        compute_stream_compositions as domain_compute_stream_compositions,
        convert_workbook_energy_unit as domain_convert_wb_energy_unit,
        convert_workbook_temperature_unit as domain_convert_wb_temp_unit,
        deserialize_bal8_workbook as domain_deserialize_bal8_workbook,
        evaluate_workbook as domain_evaluate_balance_workbook,
        iterate_target_rows as domain_iterate_balance_targets,
        preset_butane as domain_balance_preset_butane,
        preset_cuconv1 as domain_balance_preset_cuconv1,
        preset_cusmelt as domain_balance_preset_cusmelt,
        preset_fsf_process as domain_balance_preset_fsf_process,
        preset_h2o as domain_balance_preset_h2o,
        run_diagram_sweep as domain_run_balance_diagram_sweep,
        scale_stream_by_total_kg as domain_scale_stream_kg,
        scale_stream_by_total_kmol as domain_scale_stream_kmol,
        scale_stream_by_total_nm3 as domain_scale_stream_nm3,
        serialize_bal8_workbook as domain_serialize_bal8_workbook,
        set_species_amount_from_kg as domain_set_species_kg,
        set_species_amount_from_nm3 as domain_set_species_nm3,
        solve_adiabatic_temperature as domain_solve_adiabatic_temp, BalanceArea, BalanceWorkbook,
        DiagramResult as BalanceDiagramResult, SpeciesRow, StreamComposition, StreamGroup,
    },
    benson::{
        build_benson_chemical_species, estimate_benson, BensonEstimationResult, BensonParamsInput,
        MoleculeGraph,
    },
    build_reaction_export_sheets as domain_build_reaction_export_sheets, calculate,
    deserialize_rea8_workbook as domain_deserialize_rea8_workbook,
    diagrams::{calculate_diagram, DiagramRequest, DiagramResult},
    ehph::{calculate_ehph, EhPhRequest, EhPhResult},
    elements::{
        lookup_element_species as domain_lookup_element_species, ElementWorkbook,
        PeriodicDiagramData,
    },
    equilibrium::{
        solve_equilibrium_sweep, EquilibriumParams, EquilibriumSweepResult, EquilibriumSystem,
    },
    estimate::{evaluate_estimate_row, EstimateRow},
    excel_addins::{
        build_addinsample_export_sheet, build_minerals_export_sheet, build_streameq_export_sheet,
        build_webservice_formula, eval_addin_function, eval_formula_string, eval_streameq,
        lookup_mineral, migrate_legacy_formula, streameq_element_balance, AddInFunctionMeta,
        AddInUnits, ArgValue, CellEvalValue, MineralRecord, ADDIN_FUNCTION_CATALOG,
    },
    extract_reaction_plot_points as domain_extract_reaction_plot_points,
    format_reaction_sheet_tsv as domain_format_reaction_sheet_tsv,
    heat_loss::{
        solve_sheet, solve_workbook, HeatLossSheet, HeatLossWorkbook, SheetResult, WorkbookResult,
    },
    phase_stability::{
        calculate_phase_stability, PhaseStabilityDiagram, PhaseStabilityError, PhaseStabilityInput,
    },
    render_reaction_chart_svg as domain_render_reaction_chart_svg,
    serialize_rea8_workbook as domain_serialize_rea8_workbook,
    sim::{
        lca::{
            builtin_gabi_flows as domain_sim_lca_gabi_flows,
            builtin_lcia_methods as domain_sim_lca_methods,
            builtin_openlca_flows as domain_sim_lca_openlca_flows,
            preset_beta_iron_process_lca as domain_sim_lca_preset_beta_iron,
            preset_classic_zn_tsl_fuming_lca as domain_sim_lca_preset_classic_zn, BetaLcaState,
            ClassicLcaState, LcaCatalogFlow, LciaMethodSpec,
        },
        mass_balance::{
            preset_grinding_circuit_mb as domain_mb_preset_grinding,
            preset_rougher_flotation_mb as domain_mb_preset_rougher, MassBalanceWorkbook,
            MbSolution, ValidationIssue,
        },
        model::{LogSeverity, ScenarioColumn, SimProject, UnitModelCatalogEntry},
        presets::{
            builtin_unit_model_catalog as domain_sim_model_catalog,
            empty_project as domain_sim_empty_project,
            metric_value_and_unit as domain_sim_metric_value,
            preset_40_1_model as domain_sim_preset_40_1,
            preset_concentrator_flotation as domain_sim_preset_concentrator,
            preset_hydro_example as domain_sim_preset_hydro,
            preset_iron_process as domain_sim_preset_iron_process,
            preset_recycle_loop as domain_sim_preset_recycle,
            preset_zn_tsl_fuming as domain_sim_preset_zn_tsl,
        },
        reports::{
            generate_sim_reports as domain_sim_generate_reports,
            report_sheet_to_csv as domain_sim_report_csv, SimReportSheet, SimReportWorkbook,
        },
        solver::{
            clear_internal_streams as domain_sim_clear_internal,
            compute_calculation_order as domain_sim_calc_order,
            empty_all_tanks as domain_sim_empty_tanks,
            evaluate_distribution_curves as domain_sim_eval_mc_curves,
            evaluate_global_balances as domain_sim_eval_balances,
            execute_scenario as domain_sim_execute_scenario,
            export_flowsheet_svg as domain_sim_export_svg,
            export_lca_json as domain_sim_export_lca_json,
            filter_log_message as domain_sim_filter_log,
            generate_control_sweep as domain_sim_control_sweep,
            rewrite_sim7_formula as domain_sim_rewrite_sim7,
            run_control_loops as domain_sim_run_controls,
            run_static_calculation as domain_sim_run_static,
            step_dynamic_simulation as domain_sim_step_dynamic, CalcOrderInfo, LogFilterLevel,
        },
    },
    species::{
        analyze_aqueous_scales, awdb_atomic_weight, color_spec, compute_molecular_weight,
        convert_cp_function, convert_gibbs_function, extrapolate_liquid_298, fit_cp_kelley,
        gibbs_delta_to_normal, parse_hsc_formula_elements, stoichiometry_matches,
        AqueousScaleSummary, CpDataPoint, CpFitMask, CpFitResult, GibbsCoeffsInput,
        GibbsConvertOutput, HscColorSpec,
    },
    species_converter::{
        fix_element_balance, initialize_input, solve_species_conversion, CalculatedElementRow,
        ConverterInputEntry, ConverterOutputSpec, SpeciesConverterError, SpeciesConverterRequest,
        SpeciesConverterResult,
    },
    spreadsheet_export::{
        build_libreoffice_oxt, build_ods, build_onlyoffice_plugin_zip, build_xlsx,
        generate_excel_wps_vba, ExportSheet,
    },
    units::{
        build_air_humidity_3d_surface as domain_air_3d_surface,
        build_air_humidity_grid_descending as domain_air_grid_descending,
        compute_air_humidity_point as domain_air_humidity_point, AirHumidityGrid, AirHumidityPoint,
        SiPrefix, UnitsWorkbook,
    },
    water::{
        calculate_water_points as domain_calculate_water_points,
        default_water_request as domain_default_water_request,
        water_diagram_curves as domain_water_diagram_curves, WaterCalcRequest, WaterCalcResult,
        WaterDiagramCurves,
    },
    CalculationRequest, CalculationResponse, ChemicalSpecies, ReactionCollectedSheet,
    ReactionWorkbookFile, ReactionYAxis,
};

pub mod models;
pub mod ports;
pub mod testing;
pub mod use_cases;
pub use use_cases::*;
