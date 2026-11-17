//! Adaptador de catálogo en memoria para pruebas de aplicación y consumidores.

use soryntel_core::{
    ChemicalSpecies, Phase, SpeciesCatalogPort, SpeciesMetadata, SpeciesRepository,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct InMemorySpeciesCatalog {
    species: HashMap<String, ChemicalSpecies>,
    order: Vec<String>,
    metadata: HashMap<String, SpeciesMetadata>,
}

impl InMemorySpeciesCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_species(species: impl IntoIterator<Item = ChemicalSpecies>) -> Self {
        let mut catalog = Self::new();
        for species in species {
            catalog.insert_or_update_species(species);
        }
        catalog
    }

    pub fn insert(&mut self, species: ChemicalSpecies) {
        self.insert_or_update_species(species);
    }

    pub fn insert_with_metadata(&mut self, species: ChemicalSpecies, metadata: SpeciesMetadata) {
        self.metadata.insert(species.formula.clone(), metadata);
        self.insert_or_update_species(species);
    }

    fn ordered_species(&self) -> Vec<&ChemicalSpecies> {
        self.order
            .iter()
            .filter_map(|formula| self.species.get(formula))
            .collect()
    }
}

impl SpeciesRepository for InMemorySpeciesCatalog {
    fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
        self.species.get(formula).cloned()
    }

    fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
        let formula = match element_symbol {
            "H" => "H2(g)",
            "O" => "O2(g)",
            "N" => "N2(g)",
            "Cl" => "Cl2(g)",
            "F" => "F2(g)",
            "Br" => "Br2(l)",
            "I" => "I2",
            other => other,
        };
        self.get_species(formula)
    }
}

impl SpeciesCatalogPort for InMemorySpeciesCatalog {
    fn search_species(&self, query: &str, limit: usize) -> Vec<ChemicalSpecies> {
        let query = query.trim().to_lowercase();
        self.ordered_species()
            .into_iter()
            .filter(|species| {
                query.is_empty()
                    || species.formula.to_lowercase().contains(&query)
                    || species.name.to_lowercase().contains(&query)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn search_species_by_elements(
        &self,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        let elements: Vec<_> = elements.iter().map(String::as_str).collect();
        self.ordered_species()
            .into_iter()
            .filter(|species| {
                phase_is_included(
                    species.phase,
                    include_gas,
                    include_liquid,
                    include_solid,
                    include_aqua_ions,
                    include_aqua_neutrals,
                )
            })
            .filter(|species| {
                let composition = formula_elements(&species.formula);
                elements
                    .iter()
                    .all(|element| composition.iter().any(|found| found == element))
            })
            .take(limit)
            .cloned()
            .collect()
    }

    fn insert_or_update_species(&mut self, species: ChemicalSpecies) {
        if !self.species.contains_key(&species.formula) {
            let index = self
                .order
                .binary_search(&species.formula)
                .unwrap_or_else(|i| i);
            self.order.insert(index, species.formula.clone());
        }
        self.species.insert(species.formula.clone(), species);
    }

    fn species_count(&self) -> usize {
        self.species.len()
    }

    fn get_species_metadata(&self, formula: &str) -> Option<SpeciesMetadata> {
        self.metadata.get(formula).cloned()
    }

    fn get_species_at(&self, index: usize) -> Option<ChemicalSpecies> {
        self.order
            .get(index)
            .and_then(|formula| self.get_species(formula))
    }
}

fn phase_is_included(
    phase: Phase,
    gas: bool,
    liquid: bool,
    solid: bool,
    aqueous_ions: bool,
    aqueous_neutrals: bool,
) -> bool {
    match phase {
        Phase::Gas => gas,
        Phase::Liquid => liquid,
        Phase::Solid => solid,
        Phase::AqueousIon => aqueous_ions,
        Phase::AqueousNeutral => aqueous_neutrals,
    }
}

fn formula_elements(formula: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let mut current = String::new();
    for ch in formula.chars() {
        if ch.is_ascii_uppercase() {
            if !current.is_empty() {
                elements.push(std::mem::take(&mut current));
            }
            current.push(ch);
        } else if ch.is_ascii_lowercase() && !current.is_empty() {
            current.push(ch);
        } else if !current.is_empty() {
            elements.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        elements.push(current);
    }
    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::{EquilibriumUseCases, ReactionUseCases};
    use soryntel_core::{
        EnergyUnit, KelleyCoefficients, ResultFormat, SpeciesThermoInterval, TemperatureUnit,
    };
    use soryntel_thermo::equilibrium::{
        EquilibriumAmountUnit, EquilibriumParams, EquilibriumPhase, EquilibriumSpeciesInput,
        EquilibriumSystem, SolutionModel,
    };
    use soryntel_thermo::CalculationRequest;
    use std::collections::BTreeMap;

    fn species(
        formula: &str,
        phase: Phase,
        molecular_weight: f64,
        h_f_j_mol: f64,
        s_j_mol_k: f64,
        cp_j_mol_k: f64,
    ) -> ChemicalSpecies {
        ChemicalSpecies {
            formula: formula.into(),
            name: formula.into(),
            phase,
            molecular_weight,
            density_g_cm3: 1.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 200.0,
                t_max: 3000.0,
                phase,
                kelley: KelleyCoefficients {
                    a: cp_j_mol_k,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: h_f_j_mol,
                s_or_delta_s_tr: s_j_mol_k,
                reference: "Prueba en memoria".into(),
            }],
        }
    }

    fn water_catalog() -> InMemorySpeciesCatalog {
        InMemorySpeciesCatalog::with_species([
            species("H2(g)", Phase::Gas, 2.016, 0.0, 130.68, 28.84),
            species("O2(g)", Phase::Gas, 31.998, 0.0, 205.15, 29.36),
            species("H2O(g)", Phase::Gas, 18.015, -241_826.0, 188.84, 33.58),
        ])
    }

    #[test]
    fn reaction_use_case_calculates_with_only_the_memory_catalog() {
        let catalog = water_catalog();
        let request = CalculationRequest {
            input: "2H2(g) + O2(g) -> 2H2O(g)".into(),
            t_from: 25.0,
            t_to: 25.0,
            t_step: 1.0,
            temp_unit: TemperatureUnit::Celsius,
            energy_unit: EnergyUnit::Kilojoules,
            result_format: ResultFormat::Normal,
            show_transitions: false,
            criss_cobble: false,
        };
        let result = ReactionUseCases::calculate_reaction(&catalog, &request).unwrap();
        assert_eq!(result.reaction_rows.len(), 1);
        assert!(result.reaction_rows[0].delta_h < 0.0);
    }

    #[test]
    fn equilibrium_use_case_solves_with_only_the_memory_catalog() {
        let catalog = water_catalog();
        let input = |formula: &str, amount: f64| EquilibriumSpeciesInput {
            formula: formula.into(),
            feed_temp_c: 25.0,
            activity_coeff_expr: "1.0".into(),
            fixed_activity: None,
            discharge_coeff: 0.0,
            amount_unit: EquilibriumAmountUnit::Mol,
            initial_amount: amount,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        };
        let system = EquilibriumSystem {
            title: "Formación de vapor de agua".into(),
            phases: vec![EquilibriumPhase {
                name: "Gas".into(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: true,
                species: vec![
                    input("H2(g)", 0.0),
                    input("O2(g)", 0.0),
                    input("H2O(g)", 1.0),
                ],
                electrode_role: String::new(),
                capacitance_farads: 0.0,
            }],
        };
        let mut params = EquilibriumParams::default();
        params.num_indep_vars = 1;
        params.temp_range.initial = 25.0;
        params.temp_range.final_val = 25.0;
        params.temp_range.steps = 1;
        params.x_steps = 1;
        params.pressure_range.steps = 1;

        let result =
            EquilibriumUseCases::solve_equilibrium_sweep(&catalog, &system, &params).unwrap();
        assert!(!result.points.is_empty());
        assert_eq!(result.points[0].species.len(), 3);
        assert!(result.points[0].max_mass_balance_err.is_finite());
    }

    #[test]
    fn catalog_search_order_is_deterministic_by_formula() {
        let catalog = water_catalog();
        let formulas: Vec<_> = catalog
            .search_species("", 10)
            .into_iter()
            .map(|sp| sp.formula)
            .collect();
        assert_eq!(formulas, ["H2(g)", "H2O(g)", "O2(g)"]);
    }
}
