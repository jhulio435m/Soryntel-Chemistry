#!/usr/bin/env python3
"""Comprueba integridad documental y claves de presentación usadas por el escritorio."""
import hashlib
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[1]
src = root / 'crates/soryntel-desktop/src'
keys = set(json.loads((src.parent / 'locales/es.json').read_text())) - {'_version'}
used = set()
for source in src.rglob('*.rs'):
    if source.name != 'i18n.rs':
        used.update(re.findall(r'(?:i18n::text|(?<![A-Za-z_])t!)\("([^"]+)"', source.read_text()))
assert not used - keys, f'Claves de traducción ausentes: {used - keys}'
reference = root / 'docs/referencia'
manifest = json.loads((reference / 'manifest.json').read_text())
# Las copias están excluidas por el .gitignore de la raíz y solo existen en
# local; en un checkout limpio (CI) se omiten y se comprueban las presentes.
missing = []
for item in manifest:
    copy = reference / item['copia']
    if not copy.is_file():
        missing.append(item['copia'])
        continue
    assert hashlib.sha256(copy.read_bytes()).hexdigest() == item['sha256'], copy
if missing:
    print(f'Aviso: {len(missing)} de {len(manifest)} copias de referencia no están en este checkout; se omite su verificación.')
for name in ['MANUAL_USUARIO.md', 'ARQUITECTURA.md', 'REVISION.md', 'INVENTARIO.md']:
    assert (root / 'docs' / name).is_file(), name
print(f'{len(used)} claves usadas presentes; {len(manifest) - len(missing)} documentos históricos íntegros.')
print('Esta comprobación no acredita traducción completa ni validación científica de todos los módulos.')
