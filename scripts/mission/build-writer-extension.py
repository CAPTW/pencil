"""Build the product-only Writer OXT paired with an installed native library.

No registration, user-profile changes, fixture commands or document access.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path, PureWindowsPath
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--library-file', required=True, type=Path)
parser.add_argument('--installed-library', required=True)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
installed = PureWindowsPath(args.installed_library)
if not installed.is_absolute() or installed.name != 'target-apply.dll' or installed.drive.startswith('\\\\') or '..' in installed.parts:
    parser.error('installed-library must be an absolute local target-apply.dll path')
root = Path(__file__).resolve().parents[2]
base = root / 'adapters/libreoffice'
names = ('writer.py', 'Addons.xcu', 'description.xml', 'META-INF/manifest.xml',
         'pythonpath/grammar_writer_core.py', 'pythonpath/grammar_writer_bridge.py')
files = {name: (base / name).read_bytes() for name in names}
digest = hashlib.sha256(args.library_file.read_bytes()).hexdigest()
files['pythonpath/grammar_writer_installation.py'] = (
    '# Generated package binding; never supplied by a document or UNO request.\n'
    f'LIBRARY_PATH = {str(installed)!r}\nLIBRARY_SHA256 = {digest!r}\n').encode('utf-8')
for name, data in files.items():
    if name.endswith('.py'):
        ast.parse(data.decode('utf-8'))
with zipfile.ZipFile(args.output, 'x', zipfile.ZIP_DEFLATED) as archive:
    for name, data in files.items():
        archive.writestr(name, data)
print(json.dumps(dict(classification='PRODUCT_WRITER_CANDIDATE_NOT_ACCEPTANCE',
    installedLibrary=str(installed), librarySha256=digest,
    files={name: hashlib.sha256(data).hexdigest() for name, data in files.items()},
    output=str(args.output.resolve()), sha256=hashlib.sha256(args.output.read_bytes()).hexdigest()), indent=2))
