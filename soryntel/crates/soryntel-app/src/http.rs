use serde::{Deserialize, Serialize};
use soryntel_application::models::reaction::CalculationRequest;
use soryntel_application::use_cases::ReactionUseCases;
use soryntel_core::SpeciesCatalogPort;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
};

const UI_HTML: &str = include_str!("ui.html");

#[derive(Deserialize)]
struct BalanceReq {
    equation: String,
    smallest_coeff: f64,
}

#[derive(Serialize)]
struct BalanceResp {
    balanced: String,
}

#[derive(Serialize)]
struct ErrorResp {
    error: String,
}

pub(crate) fn run_server(db: Arc<dyn SpeciesCatalogPort + Send + Sync>, port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).expect("No se pudo abrir el servidor HTTP");
    println!("Soryntel Chemistry (Módulo 01: Ecuaciones de reacción) listo en: http://{addr}");
    println!(
        "Especies cargadas: {}. Presiona Ctrl+C para salir.",
        db.species_count()
    );

    for stream in listener.incoming().flatten() {
        let db_ref = Arc::clone(&db);
        std::thread::spawn(move || handle_client(stream, db_ref.as_ref()));
    }
}

fn handle_client(mut stream: TcpStream, db: &dyn SpeciesCatalogPort) {
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let (status, content_type, resp_body) = dispatch(&String::from_utf8_lossy(&buf[..n]), db);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
        resp_body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
pub(crate) fn dispatch(
    request: &str,
    db: &dyn SpeciesCatalogPort,
) -> (&'static str, &'static str, String) {
    let first_line = request.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let body = request.split("\r\n\r\n").nth(1).unwrap_or("");

    match (method, path) {
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", UI_HTML.to_string()),
        ("GET", "/api/info") => (
            "200 OK",
            "application/json",
            format!(r#"{{"species_count":{}}}"#, db.species_count()),
        ),
        ("GET", p) if p.starts_with("/api/species") => {
            let q = p.split("q=").nth(1).unwrap_or("");
            let decoded = url_decode(q);
            let list = db.search_species(&decoded, 50);
            (
                "200 OK",
                "application/json",
                serde_json::to_string(&list).unwrap(),
            )
        }
        ("POST", "/api/balance") => match serde_json::from_str::<BalanceReq>(body) {
            Ok(breq) => match ReactionUseCases::balance_reaction_equation(
                &breq.equation,
                breq.smallest_coeff,
            ) {
                Ok((balanced, _)) => (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&BalanceResp { balanced }).unwrap(),
                ),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::to_string(&ErrorResp {
                        error: e.to_string(),
                    })
                    .unwrap(),
                ),
            },
            Err(e) => (
                "400 Bad Request",
                "application/json",
                serde_json::to_string(&ErrorResp {
                    error: e.to_string(),
                })
                .unwrap(),
            ),
        },
        ("POST", "/api/calculate") => match serde_json::from_str::<CalculationRequest>(body) {
            Ok(creq) => match ReactionUseCases::calculate_reaction(db, &creq) {
                Ok(resp) => (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&resp).unwrap(),
                ),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::to_string(&ErrorResp {
                        error: e.to_string(),
                    })
                    .unwrap(),
                ),
            },
            Err(e) => (
                "400 Bad Request",
                "application/json",
                serde_json::to_string(&ErrorResp {
                    error: e.to_string(),
                })
                .unwrap(),
            ),
        },
        _ => (
            "404 Not Found",
            "text/plain",
            "Ruta no encontrada".to_string(),
        ),
    }
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(hex as char);
                i += 3;
                continue;
            }
            out.push('%');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_application::testing::InMemorySpeciesCatalog;
    use soryntel_core::{ChemicalSpecies, KelleyCoefficients, Phase, SpeciesThermoInterval};

    #[test]
    fn balance_http_preserves_response() {
        let request = "POST /api/balance HTTP/1.1\r\n\r\n{\"equation\":\"H2 + O2 = H2O\",\"smallest_coeff\":1}";
        let (status, content_type, body) = dispatch(request, &InMemorySpeciesCatalog::new());
        assert_eq!((status, content_type), ("200 OK", "application/json"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"balanced":"2H2 + O2 = 2H2O"})
        );
    }

    #[test]
    fn calculate_http_uses_catalog_port() {
        let species = |formula: &str, weight, cp, h, entropy| ChemicalSpecies {
            formula: formula.into(),
            name: formula.into(),
            phase: Phase::Gas,
            molecular_weight: weight,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![SpeciesThermoInterval {
                t_min: 200.0,
                t_max: 3000.0,
                phase: Phase::Gas,
                kelley: KelleyCoefficients {
                    a: cp,
                    b: 0.0,
                    c: 0.0,
                    d: 0.0,
                },
                h_or_delta_h_tr: h,
                s_or_delta_s_tr: entropy,
                reference: "fixture".into(),
            }],
        };
        let catalog = InMemorySpeciesCatalog::with_species([
            species("H2(g)", 2.016, 28.84, 0.0, 130.68),
            species("O2(g)", 31.998, 29.36, 0.0, 205.15),
            species("H2O(g)", 18.015, 33.58, -241_826.0, 188.84),
        ]);
        let (status, content_type, body) =
            dispatch(&calculation_request("2H2(g) + O2(g) = 2H2O(g)"), &catalog);
        assert_eq!((status, content_type), ("200 OK", "application/json"));
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body["reaction_rows"].as_array().unwrap().len(), 1);
        assert!((body["reaction_rows"][0]["delta_h"].as_f64().unwrap() + 483.652).abs() < 1e-8);
    }
}

#[cfg(test)]
pub(crate) fn calculation_request(input: &str) -> String {
    use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};
    let request = CalculationRequest {
        input: input.into(),
        t_from: 25.0,
        t_to: 25.0,
        t_step: 1.0,
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: EnergyUnit::Kilojoules,
        result_format: ResultFormat::Normal,
        show_transitions: false,
        criss_cobble: false,
    };
    format!(
        "POST /api/calculate HTTP/1.1\r\n\r\n{}",
        serde_json::to_string(&request).unwrap()
    )
}
