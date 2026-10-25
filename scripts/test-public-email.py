import importlib.util
import pathlib
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).with_name('check-public-email.py').resolve()


class PublicEmailTests(unittest.TestCase):
    def test_history_and_tags_not_just_current_tree(self):
        with tempfile.TemporaryDirectory() as d:
            def git(*args):
                return subprocess.check_output(['git', '-C', d, *args])
            git('init', '-q')
            git('config', 'user.name', 'Fixture')
            git('config', 'user.email', 'fixture@users.noreply.github.com')
            p = pathlib.Path(d) / 'sample.txt'
            p.write_text('safe fixture', encoding='utf-8')
            git('add', '.'); git('commit', '-qm', 'safe')
            def scan():
                return subprocess.run(['python', str(SCRIPT)], cwd=d, capture_output=True)
            self.assertEqual(scan().returncode, 0)
            # Construct synthetic address at runtime so the scanner does not
            # mistake the negative test source itself for personal data.
            address = 'private-person' + '@' + 'private-mail.invalid'
            p.write_text(address, encoding='utf-8')
            git('add', '.'); git('commit', '-qm', 'fixture')
            p.unlink(); git('add', '-u'); git('commit', '-qm', 'delete')
            result = scan()
            self.assertEqual(result.returncode, 1)
            self.assertNotIn(address.encode(), result.stdout + result.stderr)

    def test_author_and_annotated_tag(self):
        with tempfile.TemporaryDirectory() as d:
            def git(*args):
                return subprocess.check_output(['git', '-C', d, *args])
            git('init', '-q'); git('config', 'user.name', 'Fixture')
            address = 'private-person' + '@' + 'private-mail.invalid'
            git('config', 'user.email', address)
            git('commit', '--allow-empty', '-qm', 'private author fixture')
            git('tag', '-am', 'fixture', 'v1')
            result = subprocess.run(['python', str(SCRIPT), '--ref', 'v1'], cwd=d, capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn(address.encode(), result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
