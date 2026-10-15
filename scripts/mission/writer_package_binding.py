"""Read-only Writer package compatibility checks, not authenticity or acceptance."""
import hashlib
import json
from pathlib import Path
import re
import subprocess


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError('duplicate metadata key: ' + key)
            result[key] = value
        return result
    return json.loads(Path(path).read_text(encoding='utf-8-sig'), object_pairs_hook=unique)


def verify(package, library, source_root, files):
    package = Path(package).resolve(strict=True)
    if Path(library).resolve(strict=True) != package / 'target-apply.dll':
        raise ValueError('library-file must be the package target-apply.dll')
    manifest = read_json(package / 'MANIFEST.json')
    receipt = read_json(package / 'BUILD_RECEIPT.json')
    commit = manifest['sourceCommit']
    if not re.fullmatch('[0-9a-f]{40}', commit):
        raise ValueError('invalid source commit')
    if receipt['schemaVersion'] != 3 or receipt['after']['commit'] != commit:
        raise ValueError('source receipt mismatch')
    target = receipt['targetApply']
    if target['writerClientEnabled'] is not True:
        raise ValueError('native library was built with Writer disabled; rebuild required')
    for name, expected in [('target-apply.dll', target['librarySha256']),
                           ('codex-pencil.exe', target['brokerSha256'])]:
        rows = [row for row in manifest['files'] if row['path'] == name]
        if len(rows) != 1 or rows[0]['sha256'] != expected or sha(package / name) != expected:
            raise ValueError('package binary mismatch: ' + name)
    writer = Path(target['writerRuntimePath'])
    if not writer.is_absolute() or writer.name.lower() != 'soffice.bin' or sha(writer) != target['writerSha256']:
        raise ValueError('paired Writer executable unavailable or changed')
    # Compare the actual OXT inputs, not only the checkout HEAD. A later docs-only
    # commit can reuse the same adapter, but changed adapter code needs a rebuild.
    for name, data in files.items():
        original = subprocess.run(['git', '-C', str(source_root), 'show',
            commit + ':adapters/libreoffice/' + name], capture_output=True, check=True).stdout
        if original.replace(b'\r\n', b'\n') != data.replace(b'\r\n', b'\n'):
            raise ValueError('adapter source differs from package: ' + name)
    return dict(classification='PACKAGE_BINDING_VERIFIED_NOT_INSTALLED_ACCEPTANCE',
                sourceCommit=commit, writerPath=str(writer), writerSha256=target['writerSha256'],
                manifestSha256=sha(package / 'MANIFEST.json'),
                buildReceiptSha256=sha(package / 'BUILD_RECEIPT.json'))
