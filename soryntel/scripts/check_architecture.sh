#!/usr/bin/env bash
set -uo pipefail

mode="default"
case "${1:-}" in
  "") ;;
  --report) mode="report" ;;
  --strict) mode="strict" ;;
  *) echo "Uso: $0 [--report|--strict]" >&2; exit 2 ;;
esac

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd -- "$script_dir/.." && pwd)"
core_fail=0
application_fail=0
ui_fail=0
facade_fail=0

report_check() {
  local label="$1" status="$2" findings="$3"
  printf '%s: %s\n' "$label" "$status"
  if [[ -n "$findings" ]]; then
    while IFS= read -r line; do [[ -n "$line" ]] && printf '  %s\n' "$line"; done <<< "$findings"
  elif [[ "$status" == "PASS" ]]; then
    printf '  Sin hallazgos.\n'
  fi
}

core_tree="$(cd "$root" && cargo tree -p soryntel-core -e normal 2>&1)"
core_status=$?
core_findings="$(printf '%s\n' "$core_tree" | awk 'NR > 1' | grep -E 'soryntel-[[:alnum:]_-]+' || true)"
if (( core_status != 0 )) || [[ -n "$core_findings" ]]; then
  core_fail=1
  [[ -n "$core_findings" ]] || core_findings="cargo tree falló: $core_tree"
  report_check "a) core sin dependencias soryntel-*" FAIL "$core_findings"
else
  report_check "a) core sin dependencias soryntel-*" PASS ""
fi

application_tree="$(cd "$root" && cargo tree -p soryntel-application -e normal 2>&1)"
application_status=$?
application_findings="$(printf '%s\n' "$application_tree" | awk 'NR > 1' | grep -E 'soryntel-db|(^|[^[:alnum:]_-])(eframe|egui)(-[[:alnum:]_-]*)?([[:space:]]|$)' || true)"
if (( application_status != 0 )) || [[ -n "$application_findings" ]]; then
  application_fail=1
  [[ -n "$application_findings" ]] || application_findings="cargo tree falló: $application_tree"
  report_check "b) application sin db/eframe/egui" FAIL "$application_findings"
else
  report_check "b) application sin db/eframe/egui" PASS ""
fi

ui_findings="$(python3 - "$root" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1]) / "crates/soryntel-desktop/src"
needle = re.compile(r"soryntel_db|ThermoDatabase")
findings = {}
for path in sorted(root.rglob("*.rs")):
    if path.name in {"main.rs", "composition.rs"}:
        continue
    lines = path.read_text(errors="replace").splitlines()
    cfg_depth = 0
    cfg_pending = False
    for number, line in enumerate(lines, 1):
        if cfg_depth:
            cfg_depth += line.count("{") - line.count("}")
            if cfg_depth <= 0:
                cfg_depth = 0
            continue
        if "#[cfg(test)]" in line:
            depth = line.count("{") - line.count("}")
            if depth > 0:
                cfg_depth = depth
            else:
                cfg_pending = True
            continue
        if cfg_pending:
            depth = line.count("{") - line.count("}")
            if depth > 0:
                cfg_depth = depth
            cfg_pending = False
            continue
        if needle.search(line):
            findings.setdefault(str(path.relative_to(root)), []).append(number)
print("\n".join(f"crates/soryntel-desktop/src/{path}:{','.join(map(str, numbers))}" for path, numbers in findings.items()))
PY
)"
if [[ -n "$ui_findings" ]]; then ui_fail=1; report_check "c) UI sin referencias directas a db" FAIL "$ui_findings"; else report_check "c) UI sin referencias directas a db" PASS ""; fi

facade_result="$(python3 - "$root" <<'PYF'
from pathlib import Path
import re, sys
base = Path(sys.argv[1]) / "crates"
pattern = re.compile(r"SoryntelApplication::")
files = [base / "soryntel-desktop/src", base / "soryntel-app/src"]
lines_by_file = {}
count = 0
for directory in files:
    for path in sorted(directory.rglob("*.rs")):
        matches = []
        for number, line in enumerate(path.read_text(errors="replace").splitlines(), 1):
            occurrences = len(pattern.findall(line))
            if occurrences:
                count += occurrences
                matches.append(number)
        if matches:
            lines_by_file[str(path.relative_to(base.parent))] = matches
print(count)
for path, numbers in lines_by_file.items():
    print(f"{path}:{','.join(map(str, numbers))}")
PYF
)"
facade_count="$(printf '%s\n' "$facade_result" | head -n 1)"
facade_findings="$(printf '%s\n' "$facade_result" | tail -n +2)"
if (( facade_count > 0 )); then facade_fail=1; report_check "d) clientes sin fachada SoryntelApplication:: (conteo: $facade_count)" FAIL "$facade_findings"; else report_check "d) clientes sin fachada SoryntelApplication:: (conteo: 0)" PASS ""; fi

printf 'e) Referencias informativas a soryntel_thermo:: y soryntel_solver:: por archivo:\n'
python3 - "$root/crates/soryntel-desktop/src" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])
pattern = re.compile(r"soryntel_(?:thermo|solver)::")
for path in sorted(root.rglob("*.rs")):
    count = sum(len(pattern.findall(line)) for line in path.read_text(errors="replace").splitlines())
    if count:
        print(f"  {path.relative_to(root)}: {count}")
PY
report_check "e) conteo informativo de dependencias de dominio en desktop/src" PASS ""

if [[ "$mode" == "report" ]]; then exit 0; fi
if (( core_fail || application_fail )); then exit 1; fi
if [[ "$mode" == "strict" ]] && (( ui_fail || facade_fail )); then exit 1; fi
exit 0
