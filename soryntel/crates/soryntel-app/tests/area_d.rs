use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(id: &str) -> Self {
        let path = std::env::temp_dir().join(format!("soryntel-{id}-{}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn occupied_port_exits_without_panicking() {
    let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = occupied.local_addr().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_soryntel-app"))
        .args(["--port", &addr.port().to_string()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && !stderr.contains("panicked") && stderr.contains(&addr.to_string()),
        "exit {:?}, stderr: {stderr}",
        output.status.code()
    );
}

#[test]
#[ignore = "BUG-D04: sync-release --soft reemplaza las descargas con una lista vacía"]
fn empty_release_preserves_download_manifest() {
    let fixture = Fixture::new("d04");
    let root = &fixture.0;
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::create_dir_all(root.join("src/content")).unwrap();
    fs::copy(
        repository().join("web/scripts/sync-release.mjs"),
        root.join("scripts/sync-release.mjs"),
    )
    .unwrap();
    let manifest = root.join("src/content/release.json");
    let previous = fs::read(repository().join("web/src/content/release.json")).unwrap();
    fs::write(&manifest, &previous).unwrap();
    fs::write(
        root.join("fetch-fixture.mjs"),
        r#"globalThis.fetch = async () => new Response(JSON.stringify({
            tag_name: "v0.1.2", published_at: "2026-11-18T00:00:00Z", assets: []
        }));"#,
    )
    .unwrap();

    let output = Command::new("node")
        .current_dir(root)
        .args([
            "--import",
            "./fetch-fixture.mjs",
            "scripts/sync-release.mjs",
            "--soft",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = fs::read_to_string(&manifest).unwrap();
    assert_eq!(actual, String::from_utf8(previous).unwrap());
}

#[test]
#[ignore = "BUG-D05: la ejecución manual busca los paquetes en el directorio equivocado"]
fn manual_release_upload_finds_staged_packages() {
    let fixture = Fixture::new("d05");
    let dist = fixture.0.join("dist");
    fs::create_dir(&dist).unwrap();
    let packages = [
        "Soryntel-Chemistry-0.1.1-win64-setup.exe",
        "Soryntel-Chemistry-0.1.1-win64-portable.zip",
        "Soryntel-Chemistry-0.1.1-linux-x86_64.run",
        "Soryntel-Chemistry-0.1.1-linux-x86_64.tar.gz",
        "soryntel-chemistry-0.1.1-1-x86_64.pkg.tar.zst",
    ];
    for package in packages {
        fs::write(dist.join(package), "fixture").unwrap();
    }
    let workflow = fs::read_to_string(repository().join(".github/workflows/release.yml")).unwrap();
    let (_, step) = workflow
        .split_once("name: Comprobar los archivos que ofrece la web")
        .unwrap();
    let (_, script) = step.split_once("run: |\n").unwrap();
    let script = script
        .lines()
        .map_while(|line| line.strip_prefix("          "))
        .collect::<Vec<_>>()
        .join("\n");
    let output = Command::new("bash")
        .current_dir(&fixture.0)
        .env("VERSION", "0.1.1")
        .args(["-ec", &script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let (_, upload) = workflow
        .split_once("name: Artefactos (ejecución manual)")
        .unwrap();
    let pattern = upload
        .lines()
        .find_map(|line| line.trim().strip_prefix("path: "))
        .unwrap();
    let directory = pattern.strip_suffix("/*").unwrap();
    let missing: Vec<_> = packages
        .into_iter()
        .chain(["SHA256SUMS.txt"])
        .filter(|package| !fixture.0.join(directory).join(package).is_file())
        .collect();
    assert!(missing.is_empty(), "{pattern} no incluye {missing:?}");
}

#[test]
#[ignore = "BUG-D06: build-releases acepta un destino desconocido"]
fn unsupported_packaging_target_is_rejected() {
    let fixture = Fixture::new("d06");
    let output = Command::new("bash")
        .arg(repository().join("soryntel/packaging/build-releases.sh"))
        .arg("window")
        .env("DIST_DIR", fixture.0.join("dist"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[ignore = "BUG-D09: un cálculo de una temperatura no dibuja su único dato"]
fn single_temperature_chart_draws_a_point() {
    let script = r#"
        import { readFileSync } from 'node:fs';
        import { runInNewContext } from 'node:vm';
        const html = readFileSync(process.argv[1], 'utf8');
        const script = html.split('<script>').at(-1).split('</script>')[0];
        const svg = { innerHTML: '' };
        const document = {
            getElementById: id => id === 'chartSvg' ? svg : { value: 'h' }
        };
        runInNewContext(script.replace(/\binit\(\);\s*$/, '') + `
            lastData = {
                is_single_substance: true,
                temp_unit_label: '°C',
                substance_rows: [{ temperature: 25, h: -42 }]
            };
            renderChart();
        `, { document });
        const circles = /<circle\b[^>]*\br=["'][1-9]/.test(svg.innerHTML);
        const multipleVertices = /<polyline\b[^>]*points="[^"]+ [^"]+"/.test(svg.innerHTML);
        if (!circles && !multipleVertices) {
            throw new Error('No se dibuja un punto visible: ' + svg.innerHTML.match(/<polyline[^>]+>/));
        }
    "#;
    let output = Command::new("node")
        .args(["--input-type=module", "-e", script])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui.html"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
