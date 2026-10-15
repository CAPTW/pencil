import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid
import time
import psutil

import win32event

from owned_trial import run


class OwnedTrialTests(unittest.TestCase):
    def test_supervisor_exit_kills_owned_job(self):
        with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
            marker = Path(output) / 'child-pid'
            command = [sys.executable, '-c',
                       "import os,time,pathlib; pathlib.Path(" + repr(str(marker)) + ").write_text(str(os.getpid())); time.sleep(60)"]
            code = "from owned_trial import run; run(" + repr(command) + "," + repr(output) + ",30," + repr('Local\\GrammarCrashTest-' + uuid.uuid4().hex) + ")"
            supervisor = subprocess.Popen([sys.executable, '-c', code], cwd=Path(__file__).parent,
                                          creationflags=subprocess.CREATE_NO_WINDOW)
            try:
                end = time.monotonic() + 10
                while not marker.exists() and time.monotonic() < end:
                    time.sleep(0.05)
                self.assertTrue(marker.exists())
                child = psutil.Process(int(marker.read_text()))
                supervisor.terminate()
                supervisor.wait(5)
                child.wait(5)
                self.assertFalse(child.is_running())
            finally:
                if supervisor.poll() is None:
                    supervisor.terminate()
                    supervisor.wait(5)

    def test_scope_export_matches_marker(self):
        with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
            code = "import os,json,pathlib; p=pathlib.Path(os.environ['GRAMMAR_TRIAL_ROOT']); m=json.loads((p/'.pencil-trial-owner.json').read_text()); assert m['root']==str(p); assert os.environ['GRAMMAR_HOST_TRIAL']; assert len(m['id'])==32"
            result = run([sys.executable, "-c", code], output, 10)
            self.assertEqual(result["status"], "PASS")
            self.assertEqual(result["cleanupScope"], output)

    def test_normal_exit(self):
        with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
            result = run([sys.executable, "-c", "import ctypes; assert ctypes.windll.kernel32.GetConsoleWindow()==0; print('synthetic normal exit')"], output, 10)
            self.assertEqual(result["status"], "PASS")
            self.assertEqual(result["remainingAfterCleanup"], 0)

    def test_timeout_reaps_descendant(self):
        with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
            code = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print('child started',flush=True); time.sleep(60)"
            result = run([sys.executable, "-c", code], output, 2)
            self.assertEqual(result["status"], "TIMEOUT")
            self.assertGreaterEqual(result["remainingBeforeCleanup"], 2)
            self.assertEqual(result["remainingAfterCleanup"], 0)
            self.assertEqual(result["cleanup"], "COMPLETE")

    def test_abandoned_child_is_not_success(self):
        with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
            code = "import subprocess,sys; subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'])"
            result = run([sys.executable, "-c", code], output, 10)
            self.assertEqual(result["status"], "FAIL_LEFTOVER_CHILDREN")
            self.assertEqual(result["remainingAfterCleanup"], 0)

    def test_competing_session_rejected(self):
        name = 'Local\\GrammarTrialTest-' + uuid.uuid4().hex
        lock = win32event.CreateMutex(None, True, name)
        try:
            with tempfile.TemporaryDirectory(prefix="grammar-supervisor-") as output:
                code = "from owned_trial import run; import sys,json; print(json.dumps(run([sys.executable,'-c','raise Exception()'],sys.argv[1],2,sys.argv[2])))"
                child = subprocess.run([sys.executable, "-c", code, output, name],
                                       cwd=Path(__file__).parent, capture_output=True, text=True,
                                       timeout=10, creationflags=subprocess.CREATE_NO_WINDOW)
                self.assertEqual(child.returncode, 0, child.stderr)
                self.assertEqual(json.loads(child.stdout)["status"], "SESSION_ALREADY_RUNNING")
                self.assertFalse((Path(output) / 'trial.log').exists())
        finally:
            win32event.ReleaseMutex(lock)
            lock.Close()


if __name__ == '__main__':
    unittest.main()
