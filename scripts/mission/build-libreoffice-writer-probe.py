"""Build a fresh synthetic-only OXT against the repository Writer core."""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
files = {name: root / 'scripts/mission/libreoffice-writer-probe' / name
         for name in ('writer.py', 'hooks.py', 'description.xml', 'META-INF/manifest.xml')}
for module in ('grammar_writer_core.py', 'grammar_writer_bridge.py'):
    files['pythonpath/' + module] = root / 'adapters/libreoffice/pythonpath' / module
inputs = {}
with zipfile.ZipFile(args.output, 'x', zipfile.ZIP_DEFLATED) as archive:
    for name, path in files.items():
        data = path.read_bytes()
        if path.suffix == '.py':
            ast.parse(data.decode('utf-8'))
        archive.writestr(name, data)
        inputs[name] = hashlib.sha256(data).hexdigest()
print(json.dumps(dict(scope='SYNTHETIC_FIXTURE_NOT_PRODUCT_PACKAGE',
    output=str(args.output.resolve()), inputs=inputs,
    sha256=hashlib.sha256(args.output.read_bytes()).hexdigest()), indent=2))
