use crate::{composition, http};
use soryntel_application::models::reaction::CalculationRequest;
use soryntel_application::use_cases::ReactionUseCases;
use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};

pub(crate) fn run() {
    let args: Vec<String> = std::env::args().collect();

    // Optional external read-only .HSC database path via --hsc-db <path>
    let mut hsc_db_path: Option<String> = None;
    let mut pos_args = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--hsc-db" && i + 1 < args.len() {
            hsc_db_path = Some(args[i + 1].clone());
            i += 2;
        } else if let Some(path) = args[i].strip_prefix("--hsc-db=") {
            hsc_db_path = Some(path.to_owned());
            i += 1;
        } else {
            pos_args.push(args[i].clone());
            i += 1;
        }
    }

    let db = composition::catalog(hsc_db_path.as_deref());

    match pos_args.first().map(|s| s.as_str()) {
        Some("--help") | Some("-h") => {
            println!("Soryntel Chemistry\n  balance \"<ecuación>\"\n  calc \"<fórmula o reacción>\" [desde] [hasta] [paso]\n  --port <puerto>\n  --hsc-db <archivo>\nTemperaturas de la CLI en °C; energías en kJ.");
        }
        Some("balance") => {
            let eq = pos_args.get(1).unwrap_or_else(|| {
                eprintln!("Uso: soryntel-app balance \"<ecuación>\"");
                std::process::exit(2)
            });
            match ReactionUseCases::balance_reaction_equation(eq, 1.0) {
                Ok((b, _)) => println!("{b}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("calc") => {
            let input = pos_args.get(1).unwrap_or_else(|| {
                eprintln!("Uso: soryntel-app calc \"<fórmula o reacción>\" [desde] [hasta] [paso]");
                std::process::exit(2)
            });
            let t_from: f64 = numeric_argument(&pos_args, 2, 0.0);
            let t_to: f64 = numeric_argument(&pos_args, 3, 1000.0);
            let t_step: f64 = numeric_argument(&pos_args, 4, 100.0);
            let req = CalculationRequest {
                input: input.clone(),
                t_from,
                t_to,
                t_step,
                temp_unit: TemperatureUnit::Celsius,
                energy_unit: EnergyUnit::Kilojoules,
                result_format: ResultFormat::Normal,
                show_transitions: true,
                criss_cobble: false,
            };
            match ReactionUseCases::calculate_reaction(db.as_ref(), &req) {
                Ok(resp) => println!("{}", serde_json::to_string_pretty(&resp).unwrap()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            let port = pos_args
                .iter()
                .position(|s| s == "--port")
                .and_then(|idx| pos_args.get(idx + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8765);
            if let Err(error) = http::run_server(db, port) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
}

fn numeric_argument(args: &[String], index: usize, default: f64) -> f64 {
    match args.get(index) {
        None => default,
        Some(value) => match value.parse::<f64>() {
            Ok(number) if number.is_finite() => number,
            _ => {
                eprintln!("Valor numérico no válido: {value}");
                std::process::exit(2);
            }
        },
    }
}
