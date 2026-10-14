"""Collect installed dependency notices without credentials, caches or binaries."""
import hashlib
import tomllib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1]).resolve()
out.mkdir(exist_ok=False)
records = []


def collect(name, version, license_id, directory):
    label = re.sub(r'[^A-Za-z0-9._-]', '_', name + '-' + version)
    files = []
    # Copy only named licensing texts at the package root, never arbitrary assets.
    for source in sorted(directory.iterdir()):
        if source.is_file() and re.match(r'(?i)^(licen[sc]e|copying|notice)([._-].*)?$', source.name):
            if source.stat().st_size > 2_000_000:
                raise ValueError('Unexpected notice size: ' + name)
            dest = out / label
            dest.mkdir(exist_ok=True)
            shutil.copyfile(source, dest / source.name)
            files.append(label + '/' + source.name)
    if not files:
        supplement = root / 'docs/third-party' / name.removeprefix('rust-')
        if supplement.exists():
            for source in supplement.iterdir():
                dest = out / label
                dest.mkdir(exist_ok=True)
                shutil.copyfile(source, dest / source.name)
                files.append(label + '/' + source.name)
        elif name.startswith('npm-'):
            # These optional native tools are build-time only; parent distributions
            # already supply their root notices in this collection.
            if not any(s in name for s in ('@esbuild/', '@rollup/', '@tauri-apps/cli-')):
                raise ValueError('Missing dependency notice: ' + name)
        elif name == 'rust-tauri-plugin':
            for source in directory.parent.glob('tauri-2.*/LICENSE*'):
                dest = out / label
                dest.mkdir(exist_ok=True)
                shutil.copyfile(source, dest / source.name)
                files.append(label + '/' + source.name)
        else:
            raise ValueError('Missing dependency notice: ' + name)
    records.append(dict(name=name, version=version, license=license_id, texts=files))


metadata = json.loads(subprocess.check_output(
    ['cargo', 'metadata', '--locked', '--offline', '--filter-platform', 'x86_64-pc-windows-msvc', '--format-version', '1',
     '--manifest-path', str(root / 'src-tauri/Cargo.toml')], cwd=root))
for package in metadata['packages']:
    if package['source']:
        collect('rust-' + package['name'], package['version'], package['license'],
                Path(package['manifest_path']).parent)
# Provide the exact unmodified MPL-covered selectors crate source with checksum.
locked = tomllib.loads((root / 'src-tauri/Cargo.lock').read_text())
for package in metadata['packages']:
    if package['name'] == 'selectors':
        directory = Path(package['manifest_path']).parent
        archive = directory.parents[1].parent / 'cache' / directory.parent.name / (directory.name + '.crate')
        checksum = next(p['checksum'] for p in locked['package'] if p['name'] == 'selectors' and p['version'] == package['version'])
        assert hashlib.sha256(archive.read_bytes()).hexdigest() == checksum
        shutil.copyfile(archive, out / archive.name)
lock = json.loads((root / 'package-lock.json').read_text(encoding='utf-8'))
for relative, package in lock['packages'].items():
    if not relative:
        continue
    directory = root / relative
    info = json.loads((directory / 'package.json').read_text(encoding='utf-8')) if directory.exists() else None
    # Platform-optional packages absent from this installation are not redistributed.
    if info:
        collect('npm-' + info['name'], info['version'], info.get('license'), directory)
(out / 'INDEX.json').write_text(json.dumps(records, indent=2), encoding='utf-8')
(out / 'README.txt').write_text(
    'Third-party notices from the locked build dependency installation. Includes\n'
    'build-time and target-optional Rust dependencies as a conservative superset.\n'
    'An empty texts list means no root notice was provided by that package; see\n'
    'its license identifier and upstream package. No project license is invented.\n'
    'Microsoft release CRT DLLs are private redistributable components from the\n'
    'licensed Visual Studio build tools. Gemma/llama.cpp/CUDA are downloaded\n'
    'separately, under their upstream terms, and are not in this archive.\n', encoding='utf-8')
