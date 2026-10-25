"""Owned synthetic server must survive aborted HTTP requests; no model/GPU/UI."""
import os
import json
import socket
import struct
import sys
import subprocess
import tempfile
import time
import unittest
import urllib.request
from pathlib import Path

if os.name == "nt":
    from owned_build import installed_helper
    from owned_trial import run as run_owned


@unittest.skipUnless(os.name == "nt", "Windows fixture")
class StartupFixture(unittest.TestCase):
    def test_completed_compiler_with_inherited_output_child_is_owned(self):
        with tempfile.TemporaryDirectory(prefix="grammar-compiler-child-") as tmp:
            root = Path(tmp)
            script = root / "compiler.py"
            script.write_text(
                "import subprocess, sys\n"
                "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])\n",
                encoding="utf-8")
            # Synthetic compiler exits while its declared helper retains output
            # handles. This must be explicit reaping, not EOF-based timeout or
            # a false natural-exit claim. Only this Job's children are affected.
            result = run_owned([sys.executable, str(script)], root / "owned", 60,
                               owned_helper_images=[sys.executable])
            self.assertEqual(result["status"], "BUILD_COMPLETE_HELPERS_REAPED")
            self.assertFalse(result["naturalChildExit"])
            self.assertEqual(result["cleanup"], "COMPLETE")
            self.assertEqual(result["remainingAfterCleanup"], 0)

    def test_abandoned_requests_do_not_kill_warmup_server(self):
        with tempfile.TemporaryDirectory(prefix="grammar-fixture-reset-") as tmp:
            root = Path(tmp)
            exe = root / "server.exe"
            source = Path(__file__).parent / "fixtures/gemma-startup-runtime.rs"
            # Wait on the compiler process, not EOF from pipes inherited by
            # toolchain descendants. Reuse the existing build-only Job owner;
            # a declared MSVC helper may be reaped, never counted as natural exit.
            # The actual compilation deadline remains 60 seconds.
            compiled = run_owned(["rustc", str(source), "-o", str(exe)],
                                 root / "compile", 60,
                                 owned_helper_images=[installed_helper()])
            print("fixture compile: " + json.dumps(compiled), flush=True)
            if compiled["status"] not in ("PASS", "BUILD_COMPLETE_HELPERS_REAPED"):
                print((root / "compile/trial.log").read_text(encoding="utf-8", errors="replace")[-8000:], flush=True)
            self.assertIn(compiled["status"], ("PASS", "BUILD_COMPLETE_HELPERS_REAPED"))
            self.assertEqual(compiled["cleanup"], "COMPLETE")
            self.assertEqual(compiled["remainingAfterCleanup"], 0)
            model = root / "invalid.gguf"
            model.write_text("invalid")
            with socket.socket() as reserve:
                reserve.bind(("127.0.0.1", 0))
                port = reserve.getsockname()[1]
            child = subprocess.Popen([str(exe), "-m", str(model), "--port", str(port)],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                     creationflags=0x08000000)
            try:
                deadline = time.monotonic() + 5
                while True:
                    try:
                        probe = socket.create_connection(("127.0.0.1", port), timeout=.2)
                        probe.close()  # clean EOF before headers
                        break
                    except OSError:
                        if child.poll() is not None or time.monotonic() >= deadline:
                            self.fail("fixture did not listen")
                        time.sleep(.02)
                for partial in [b"GET /health HTTP/1.1", b"POST /completion HTTP/1.1\r\nContent-Length: 20\r\n\r\n{}"]:
                    with socket.create_connection(("127.0.0.1", port), timeout=1) as abandoned:
                        abandoned.sendall(partial)
                        abandoned.setsockopt(socket.SOL_SOCKET, socket.SO_LINGER, struct.pack("HH", 1, 0))
                    time.sleep(.05)
                opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
                with opener.open(f"http://127.0.0.1:{port}/health", timeout=3) as response:
                    self.assertEqual(response.read(), b"{}")
                request = urllib.request.Request(f"http://127.0.0.1:{port}/completion", data=b"{}")
                with opener.open(request, timeout=3) as response:
                    self.assertEqual(response.read(), b"{}")
                self.assertTrue(model.with_suffix(".warmup").exists())
                self.assertIsNone(child.poll())
            finally:
                if child.poll() is None:
                    child.terminate()
                child.communicate(timeout=5)


if __name__ == "__main__":
    unittest.main()
