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
parser.add_argument('--enable-saved-docx', action='store_true', help='explicit candidate qualification switch; default off')
parser.add_argument('--enable-uniform-format', action='store_true', help='explicit bounded uniform-format candidate switch; default off')
parser.add_argument('--enable-mixed-surroundings', action='store_true', help='single homogeneous selection within one mixed-format paragraph; default off')
parser.add_argument('--enable-paragraphs', action='store_true', help='one homogeneous selection within a paragraph of a bounded multi-paragraph document; default off')
parser.add_argument('--enable-cross-format', action='store_true', help='single-paragraph replacement uses first selected style; Undo restores runs; default off')
args = parser.parse_args()
if args.enable_cross_format and not args.enable_paragraphs:
    parser.error('cross format requires --enable-paragraphs')
if args.enable_paragraphs and not args.enable_mixed_surroundings:
    parser.error('paragraphs requires --enable-mixed-surroundings')
if args.enable_mixed_surroundings and not args.enable_uniform_format:
    parser.error('mixed surroundings requires --enable-uniform-format')
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
    f'LIBRARY_PATH = {str(installed)!r}\nLIBRARY_SHA256 = {digest!r}\n'
    f'ALLOW_SAVED_DOCX = {args.enable_saved_docx!r}\nALLOW_UNIFORM_FORMAT = {args.enable_uniform_format!r}\n'
    f'ALLOW_MIXED_FORMAT = {args.enable_mixed_surroundings!r}\nALLOW_PARAGRAPHS = {args.enable_paragraphs!r}\n'
    f'ALLOW_CROSS_FORMAT = {args.enable_cross_format!r}\n').encode('utf-8')
for name, data in files.items():
    if name.endswith('.py'):
        ast.parse(data.decode('utf-8'))
with zipfile.ZipFile(args.output, 'x', zipfile.ZIP_DEFLATED) as archive:
    for name, data in files.items():
        archive.writestr(name, data)
print(json.dumps(dict(classification='PRODUCT_WRITER_CANDIDATE_NOT_ACCEPTANCE',
    savedDocxEnabled=args.enable_saved_docx, uniformFormatEnabled=args.enable_uniform_format,
    mixedSurroundingsEnabled=args.enable_mixed_surroundings,
    paragraphsEnabled=args.enable_paragraphs, crossFormatEnabled=args.enable_cross_format,
    installedLibrary=str(installed), librarySha256=digest,
    files={name: hashlib.sha256(data).hexdigest() for name, data in files.items()},
    output=str(args.output.resolve()), sha256=hashlib.sha256(args.output.read_bytes()).hexdigest()), indent=2))
