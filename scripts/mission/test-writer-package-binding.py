"""Real file/hash checks; fake binary bytes, no installed/runtime qualification."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from writer_package_binding import verify, sha, read_json


class BindingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='pencil-writer-binding-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ('target-apply.dll', 'codex-pencil.exe', 'soffice.bin'):
            (self.root / name).write_bytes(('synthetic ' + name).encode())
        self.target = dict(writerClientEnabled=True,
            writerRuntimePath=str(self.root / 'soffice.bin'), writerSha256=sha(self.root / 'soffice.bin'),
            librarySha256=sha(self.root / 'target-apply.dll'), brokerSha256=sha(self.root / 'codex-pencil.exe'))
        self.receipt = dict(schemaVersion=3, after=dict(commit='a'*40), targetApply=self.target)
        self.manifest = dict(sourceCommit='a'*40, files=[dict(path=n, sha256=sha(self.root/n))
            for n in ('target-apply.dll', 'codex-pencil.exe')])
        self.write()
        self.git = patch('writer_package_binding.subprocess.run',
            return_value=subprocess.CompletedProcess([], 0, stdout=b'original\n'))
        self.git.start()
        self.addCleanup(self.git.stop)

    def write(self):
        (self.root/'MANIFEST.json').write_text(json.dumps(self.manifest))
        (self.root/'BUILD_RECEIPT.json').write_text(json.dumps(self.receipt))

    def check(self, data=b'original\r\n'):
        return verify(self.root, self.root/'target-apply.dll', self.root, {'writer.py': data})

    def test_matching_binding_and_checkout_line_endings(self):
        self.assertEqual(self.check()['sourceCommit'], 'a'*40)

    def test_writer_disabled(self):
        self.target['writerClientEnabled'] = False
        self.write()
        with self.assertRaisesRegex(ValueError, 'Writer disabled'): self.check()

    def test_changed_binaries(self):
        for name in ('target-apply.dll', 'codex-pencil.exe', 'soffice.bin'):
            with self.subTest(name=name):
                path = self.root/name
                original = path.read_bytes()
                path.write_bytes(b'changed')
                with self.assertRaises(ValueError): self.check()
                path.write_bytes(original)

    def test_duplicate_binary_row(self):
        self.manifest['files'].append(self.manifest['files'][0])
        self.write()
        with self.assertRaisesRegex(ValueError, 'binary mismatch'): self.check()

    def test_source_mismatch(self):
        with self.assertRaisesRegex(ValueError, 'adapter source differs'): self.check(b'changed')
        self.receipt['after']['commit'] = 'b'*40
        self.write()
        with self.assertRaisesRegex(ValueError, 'source receipt mismatch'): self.check()

    def test_duplicate_metadata_key(self):
        path = self.root/'duplicate.json'
        path.write_text('{"enabled":false,"enabled":true}')
        with self.assertRaisesRegex(ValueError, 'duplicate metadata'): read_json(path)


if __name__ == '__main__': unittest.main()
