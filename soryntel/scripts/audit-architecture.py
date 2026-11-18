#!/usr/bin/env python3
"""Comprueba dependencias de las capas internas y el adaptador de reacciones."""
from pathlib import Path
import re
try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11 (p. ej. ubuntu-22.04 en CI)
    tomllib = None


def dependency_names(text):
    """Nombres de [dependencies] de un Cargo.toml."""
    if tomllib is not None:
        return set(tomllib.loads(text).get('dependencies', {}))
    names, inside = set(), False
    for line in text.splitlines():
        line = line.strip()
        if line.startswith('['):
            inside = line == '[dependencies]'
        elif inside and (m := re.match(r'([A-Za-z0-9_-]+)\s*(?:\.[A-Za-z]+\s*)?=', line)):
            names.add(m.group(1))
    return names
root = Path(__file__).resolve().parents[1]
allowed = {
    'soryntel-core': set(),
    'soryntel-solver': {'soryntel-core'},
    'soryntel-thermo': {'soryntel-core', 'soryntel-solver'},
    'soryntel-application': {'soryntel-core', 'soryntel-solver', 'soryntel-thermo'},
}
for name, permitted in allowed.items():
    deps = dependency_names((root / 'crates' / name / 'Cargo.toml').read_text())
    internal = {k for k in deps if k.startswith('soryntel-')}
    assert internal <= permitted, f'{name}: dependencia interna incorrecta {internal - permitted}'
    assert not {'eframe', 'egui', 'rusqlite'} & deps, name
ui = (root / 'crates/soryntel-desktop/src/reaction_ui.rs').read_text().split('#[cfg(test)]')[0]
assert 'ThermoDatabase' not in ui, 'La vista de reacciones depende del adaptador concreto'
cli = (root / 'crates/soryntel-app/src/main.rs').read_text()
assert 'soryntel_solver::' not in cli and 'calculate(&' not in cli
print('Dependencias internas y adaptadores del módulo 1: correctos.')
print('Revisión pendiente: I/O de IPC en dominio LCA y llamadas directas de otros módulos al dominio.')
