//! Data contracts for adapters, grouped by application capability.
//! Calculations are exposed separately through the use cases.

pub mod reaction {
    pub use soryntel_solver::{ParsedReaction, ParsedTerm};
    pub use soryntel_thermo::{
        CalculationRequest, CalculationResponse, ReactionCollectedSheet, ReactionWorkbookFile,
        ReactionYAxis,
    };
    pub use soryntel_thermo::{ReactionRow, SpeciesDataRow, SubstanceRow};
}

pub mod equilibrium {
    pub use soryntel_thermo::equilibrium::model::CellPhaseType;
    pub use soryntel_thermo::equilibrium::model::{
        AxisRange, CalculationMode, EquilibriumAmountUnit, EquilibriumParams, EquilibriumPhase,
        EquilibriumPointResult, EquilibriumSpeciesInput, EquilibriumSweepResult, EquilibriumSystem,
        EquilibriumWorkbookFile, ExtrapolationWarning, SolutionModel, SpeciesEquilibriumResult,
        StepMode, SweepVariable, TargetSpec, ThermodynamicMode,
    };
}

pub mod balance {
    pub use soryntel_thermo::balance::{H2O_PC_BAR, H2O_TC_K};
    pub use soryntel_thermo::balance_engine::{
        AreaBalanceSummary, DiagramConfig, DiagramSeriesDef, SheetTotals, StreamComposition,
        StreamElementRow,
    };
    pub use soryntel_thermo::balance_engine::{
        BalanceArea, BalanceUserSheet, BalanceWorkbook, DiagramResult, ElementBalanceRow,
        SpeciesRow, StreamGroup, TargetRow,
    };
}

pub mod heat_loss {
    pub use soryntel_thermo::heat_loss::convection::ConvectionDiagnostics;
    pub use soryntel_thermo::heat_loss::diagram::{
        OptimumInsulationPoint, OptimumInsulationSpec, SweepParameter, SweepPoint,
        TemperatureProfileSeries,
    };
    pub use soryntel_thermo::heat_loss::gas_radiation::{
        GasComposition, GasRadiationInput, GasRadiationResult, OpticalDepthGeometry,
        ParticleSuspensionDetailedResult, ParticleSuspensionSpec,
    };
    pub use soryntel_thermo::heat_loss::gas_radiation::{ParticleCloudInput, SpectralBandRow};
    pub use soryntel_thermo::heat_loss::material_db::{
        MaterialCompositionRow, MaterialCpRange, MaterialDatabase, MaterialUserProperty,
    };
    pub use soryntel_thermo::heat_loss::material_db::{
        MaterialPropertyRow, MaterialRecord, ResolvedElementWt, ResolvedSpeciesWt,
    };
    pub use soryntel_thermo::heat_loss::model::{
        CalculationMode, ColumnType, ConvectionFunctionPreset, ConvectionGeometry,
        ConvectionRegime, FluidPropertyTable, Geometry, HeatLossColumn, HeatLossWorkbook,
        SolidConductivityData, SurfaceEmissivityData, WorkbookResult,
    };
    pub use soryntel_thermo::heat_loss::model::{
        ColumnResult, ConvectionFunctionTerm, FluidPropertiesPoint, HeatLossSheet, SheetResult,
    };
    pub use soryntel_thermo::heat_loss::target::TargetResult;
    pub use soryntel_thermo::heat_loss::target::{TargetGoal, TargetSpec, TargetVariable};
}

pub mod exergy {
    pub use soryntel_thermo::exergy::{ExergySide, ExergyStreamKind, ExergyWorkbook};
    pub use soryntel_thermo::exergy::{ExergySpeciesRow, ExergyStream};
}

pub mod benson {
    pub use soryntel_thermo::benson::{
        AtomNode, BensonGroupEntry, BensonGroupParams, BondEdge, StructuralBlockSpec,
    };
    pub use soryntel_thermo::benson::{
        BensonEstimationResult, BensonParamsInput, BlockCategory, BondOrder, ElementKind,
        MoleculeGraph, PhaseThermoSummary,
    };
}

pub mod diagrams {
    pub use soryntel_thermo::diagrams::{
        DiagramEnergyUnit, DiagramRequest, DiagramResult, DiagramSeries, DiagramSpeciesInput,
        DiagramType,
    };
}

pub mod ehph {
    pub use soryntel_thermo::ehph::{
        BoundarySegment, EhPhRequest, EhPhResult, ElementSpec, PotentialScale,
    };
}

pub mod phase_stability {
    pub use soryntel_thermo::phase_stability::{
        AxisVariable, PhaseStabilityDiagram, PhaseStabilityInput, TemperatureUnit,
    };
    pub use soryntel_thermo::phase_stability::{
        PhaseStabilityError, PhaseStabilitySpeciesRow, StabilityBoundarySegment,
        StabilityLegendEntry, StabilityRegionLabel,
    };
}

pub mod units {
    pub use soryntel_thermo::units::{
        AirHumidityGrid, AirHumidityPoint, ConvertRow, SiPrefix, UnitsWorkbook, STD_ATM_BAR,
    };
}

pub mod simulation {
    pub use soryntel_thermo::sim::model::{
        AnnotationKind, ArrowHeadStyle, DefaultCanvasSettings, LineStyle, LockState, LogSeverity,
        ModelImplType, ModelLibraryTab, MonteCarloDist, SankeyColorMode, SimAnnotation, SimGroup,
        SimLayer, SimPage, SimProject, SimStream, SimTableObj, SimUnit, StreamConnectivity,
        StreamPhaseKind, StreamTableSet, TableViewMode, UnitBehavior, UnitKind,
        UnitModelCatalogEntry,
    };
    pub use soryntel_thermo::sim::model::{ConvergenceRound, ScenarioColumn};
    pub use soryntel_thermo::sim::presets::{
        UnitIconMeta, DEFAULT_STREAM_VARIABLES, UNIT_ICON_GALLERY, VISUALIZATION_METRICS,
    };
    pub use soryntel_thermo::sim::reports::SimReportSheet;
    pub use soryntel_thermo::sim::reports::SimReportWorkbook;
    pub use soryntel_thermo::sim::solver::CalcOrderInfo;
    pub use soryntel_thermo::sim::solver::LogFilterLevel;
}

pub mod mass_balance {
    pub use soryntel_thermo::sim::mass_balance::{
        BalanceType, CellKey, ChartPanelSpec, CheckSeverity, MassBalanceWorkbook, MbSolution,
        MbStream, MbVariable, MeasurementMethod, PostBalStatus, PreMeasStatus, SampleQuality,
        SizingMode, SolveMethod, StreamEquation, StreamType, ValidationIssue, VarTypeCode,
    };
}

pub mod lca {
    pub use soryntel_thermo::sim::lca::{
        BetaLcaMode, BetaLcaState, BetaStreamRow, ClassicLcaState, ClassicProcessInfo,
        LcaCatalogFlow, LcaDatabaseKind, LcaFlowType, LcaFlowUnitKind, LcaGroup, LciaMethodSpec,
        BUILTIN_LCA_NOTICE,
    };
}

pub mod aqua {
    pub use soryntel_thermo::aqua::{
        ActiveParameterSetsReport, AquaCalcResult, AquaCaseFile, AquaEnergyUnit, AquaMethod,
        AquaPressUnit, AquaSpeciesInput, AquaTempUnit, DensityCalcResult, DensityInputRow,
        MergedPitzerCatalog, OwnBinaryRow, OwnLambdaRow, OwnPitzerDb, OwnPsiRow, OwnThetaRow,
        PitzerSourceColor,
    };
    pub use soryntel_thermo::aqua::{
        AquaPhaseSummary, AquaSpeciesResult, DensityRowViolation, MergedBinaryMeta,
        MergedLambdaMeta, MergedPsiMeta, MergedThetaMeta,
    };
    pub use soryntel_thermo::aqua_density_data::DENSITY_SALTS;
    pub use soryntel_thermo::aqua_pitzer_data::{
        BinaryPitzerEntry, LambdaPitzerEntry, PsiPitzerEntry, ThetaPitzerEntry,
    };
    pub use soryntel_thermo::aqua_pitzer_data::{PitzerDatabase, PitzerTempCurve, ValidityRange};
}

pub mod water {
    pub use soryntel_thermo::balance::{H2O_PC_BAR, H2O_TC_K};
    pub use soryntel_thermo::water::{IsentropicSummary, WaterPointInput, WaterPointOutput};
    pub use soryntel_thermo::water::{
        WaterAmountUnit, WaterCalcRequest, WaterCalcResult, WaterDensityUnit, WaterDiagramCurves,
        WaterEnergyUnit, WaterPhaseMode, WaterPressureUnit, WaterTempUnit, H2O_MOL_WEIGHT,
        H2O_PT_BAR, H2O_TT_K,
    };
}

pub mod species {
    pub use soryntel_thermo::species::{AqueousScaleSummary, HscColorSpec};
    pub use soryntel_thermo::species::{
        CpDataPoint, CpFitMask, CpFitResult, ExternalCpTemplate, GibbsCoeffsInput,
        GibbsConvertOutput, HSC_COLOR_PALETTE,
    };
}

pub mod elements {
    pub use soryntel_thermo::elements::{
        ElementCategory, ElementWorkbook, PeriodicDiagramData, StandardState, PERIODIC_ELEMENTS,
    };
}

pub mod estimate {
    pub use soryntel_thermo::estimate::{DatabaseRefValues, ElementInfo, EstimateParityStats};
    pub use soryntel_thermo::estimate::{
        EstimateRow, EstimateSpeciesType, FunctionalGroup, OrganicGroup,
    };
}

pub mod species_converter {
    pub use soryntel_thermo::species_converter::SpeciesConverterError;
    pub use soryntel_thermo::species_converter::{
        ConversionMethod, ConverterInputEntry, ConverterOutputSpec, ElementBalanceRow,
        SpeciesConverterRequest, SpeciesConverterResult, SpeciesTypeFilter,
    };
}

pub mod addins {
    pub use soryntel_thermo::excel_addins::{
        AddInFunctionMeta, AddInOrigin, AddInUnits, ArgValue, CellEvalValue, MineralRecord,
        ADDIN_FUNCTION_CATALOG,
    };
    pub use soryntel_thermo::spreadsheet_export::{
        ExportCell, ExportCellStyle, ExportCellValue, ExportSheet,
    };
}
