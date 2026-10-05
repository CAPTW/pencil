"""Package boundary tests; no LibreOffice or installed acceptance claim."""
import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

builder = Path(__file__).with_name('build-writer-extension.py')


class Tests(unittest.TestCase):
    def test_product_only_archive_binds_exact_library(self):
        with tempfile.TemporaryDirectory(prefix='grammar-writer-package-') as root:
            root = Path(root)
            library = root / 'target-apply.dll'
            library.write_bytes(b'synthetic packaging input, not an executable')
            output = root / 'product.oxt'
            command = [sys.executable, str(builder), '--library-file', str(library),
                '--installed-library', r'C:\GrammarTrial\target-apply.dll', '--output', str(output)]
            completed = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            receipt = json.loads(completed.stdout)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(set(archive.namelist()), set(receipt['files']))
                self.assertFalse(any('hooks' in name or 'fixture' in name for name in archive.namelist()))
                entry = archive.read('writer.py').decode()
                self.assertNotIn('org.grammar.fixture', entry)
                self.assertNotIn('XJob)', entry)
                values = {}
                tree = ast.parse(archive.read('pythonpath/grammar_writer_installation.py').decode())
                for statement in tree.body:
                    values[statement.targets[0].id] = ast.literal_eval(statement.value)
                self.assertEqual(values['LIBRARY_PATH'], r'C:\GrammarTrial\target-apply.dll')
                self.assertIs(values['ALLOW_SAVED_DOCX'], False)
                self.assertIs(values['ALLOW_UNIFORM_FORMAT'], False)
                self.assertIs(values['ALLOW_MIXED_FORMAT'], False)
                self.assertIs(receipt['mixedSurroundingsEnabled'], False)
                self.assertIs(receipt['uniformFormatEnabled'], False)
                self.assertIs(receipt['savedDocxEnabled'], False)
                self.assertEqual(values['LIBRARY_SHA256'], hashlib.sha256(library.read_bytes()).hexdigest())
                for name in archive.namelist():
                    self.assertEqual(hashlib.sha256(archive.read(name)).hexdigest(), receipt['files'][name])
            original = output.read_bytes()
            retry = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(retry.returncode, 0)
            self.assertEqual(output.read_bytes(), original)

    def test_untrusted_install_locations_rejected_before_output(self):
        for location in ('target-apply.dll', r'\\server\share\target-apply.dll', r'C:\one\..\target-apply.dll', r'C:\one\evil.dll'):
            with self.subTest(location=location), tempfile.TemporaryDirectory(prefix='grammar-writer-package-') as root:
                output = Path(root) / 'product.oxt'
                completed = subprocess.run([sys.executable, str(builder), '--library-file', str(Path(root) / 'missing.dll'),
                    '--installed-library', location, '--output', str(output)], capture_output=True, text=True)
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn('installed-library', completed.stderr)
                self.assertFalse(output.exists())


    def test_saved_docx_requires_explicit_package_option(self):
        with tempfile.TemporaryDirectory(prefix='grammar-writer-saved-package-') as root:
            root = Path(root)
            library = root / 'target-apply.dll'
            library.write_bytes(b'synthetic packaging input')
            output = root / 'saved.oxt'
            completed = subprocess.run([sys.executable, str(builder), '--library-file', str(library),
                '--installed-library', r'C:\GrammarTrial\target-apply.dll', '--output', str(output),
                '--enable-saved-docx'], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertIs(json.loads(completed.stdout)['savedDocxEnabled'], True)
            with zipfile.ZipFile(output) as archive:
                tree = ast.parse(archive.read('pythonpath/grammar_writer_installation.py').decode())
                values = {node.targets[0].id: ast.literal_eval(node.value) for node in tree.body}
                self.assertIs(values['ALLOW_SAVED_DOCX'], True)


    def test_uniform_format_requires_explicit_package_option(self):
        with tempfile.TemporaryDirectory(prefix='grammar-writer-format-package-') as root:
            root = Path(root)
            library = root / 'target-apply.dll'
            library.write_bytes(b'synthetic packaging input')
            output = root / 'formatted.oxt'
            completed = subprocess.run([sys.executable, str(builder), '--library-file', str(library),
                '--installed-library', r'C:\GrammarTrial\target-apply.dll', '--output', str(output),
                '--enable-uniform-format'], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            receipt = json.loads(completed.stdout)
            self.assertIs(receipt['uniformFormatEnabled'], True)
            self.assertIs(receipt['savedDocxEnabled'], False)
            with zipfile.ZipFile(output) as archive:
                tree = ast.parse(archive.read('pythonpath/grammar_writer_installation.py').decode())
                values = {node.targets[0].id: ast.literal_eval(node.value) for node in tree.body}
                self.assertIs(values['ALLOW_UNIFORM_FORMAT'], True)
                self.assertIs(values['ALLOW_SAVED_DOCX'], False)


    def test_mixed_requires_uniform_and_explicit_option(self):
        with tempfile.TemporaryDirectory(prefix='grammar-writer-mixed-package-') as root:
            root = Path(root)
            library = root / 'target-apply.dll'
            library.write_bytes(b'synthetic packaging input')
            output = root / 'mixed.oxt'
            command = [sys.executable, str(builder), '--library-file', str(library),
                '--installed-library', r'C:\GrammarTrial\target-apply.dll', '--output', str(output),
                '--enable-mixed-surroundings']
            rejected = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertFalse(output.exists())
            enabled = subprocess.run(command + ['--enable-uniform-format'], capture_output=True, text=True)
            self.assertEqual(enabled.returncode, 0, enabled.stderr)
            self.assertTrue(json.loads(enabled.stdout)['mixedSurroundingsEnabled'])
            with zipfile.ZipFile(output) as archive:
                tree = ast.parse(archive.read('pythonpath/grammar_writer_installation.py').decode())
                values = {node.targets[0].id: ast.literal_eval(node.value) for node in tree.body}
                self.assertIs(values['ALLOW_MIXED_FORMAT'], True)
                self.assertIs(values['ALLOW_UNIFORM_FORMAT'], True)
                self.assertIs(values['ALLOW_SAVED_DOCX'], False)


if __name__ == '__main__': unittest.main()
