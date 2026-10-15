"""Actual built broker + production launcher + owned synthetic runtime/app; no GPU/UI."""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import time
import unittest

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("launcher", HERE / "test-gemma-launcher.py")
launcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(launcher)


class ProductionBrokerTests(launcher.LauncherTests):
    def test_artifact_failure_is_retryable_without_claiming_runtime_cleanup(self):
        broker = launcher.ROOT / "src-tauri/target/debug/grammar-gemma-broker.exe"
        self.assertTrue(broker.is_file())
        shutil.copyfile(broker, self.root / broker.name)
        runtime = self.root / "local/runtime/llama-server.exe"
        subprocess.run(["rustc", str(HERE / "fixtures/gemma-startup-runtime.rs"), "-o", str(runtime)], check=True, capture_output=True, creationflags=0x08000000)
        model = self.root / "local/gemma-4-12b-it-qat-q4_0.gguf"
        model.write_text("invalid")
        catalog_path = self.root / "Gemma-Assets.json"
        catalog = json.loads(catalog_path.read_text())
        catalog["runtimeSha256"] = launcher.digest(runtime)
        catalog["runtimeBundleSha256"] = launcher.hashlib.sha256(("llama-server.exe\0" + launcher.digest(runtime) + "\n").encode()).hexdigest()
        catalog["model"]["sha256"] = "0" * 64
        catalog_path.write_text(json.dumps(catalog))
        for entry in self.manifest["files"]:
            path = self.root / entry["path"]
            entry.update(sha256=launcher.digest(path), bytes=path.stat().st_size)
        (self.root / "MANIFEST.json").write_text(json.dumps(self.manifest))
        (self.root / "wait-release").touch()
        command = "[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); try { & '" + str(self.root / "Start-Grammar.ps1").replace("'", "''") + "' -EnableDesktopGemma } finally { foreach ($failure in $Error) { [Console]::Error.WriteLine($failure.ToString()) } }"
        process = subprocess.Popen(["pwsh", "-NoProfile", "-Command", command], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8", creationflags=0x08000000)
        try:
            path, state = self.wait_phase("failed")
            self.assertIsNotNone(state["retryTicket"])
            self.assertFalse(model.with_suffix(".pid").exists(), "runtime must never launch")
            model.write_text("invalid")  # artifact pins released before retry admission
            app_pid = (self.root / "app.pid").read_text()
            path.with_suffix(".json.retry").write_text(state["retryTicket"])
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                current = json.loads(path.read_text())
                if current["phase"] == "failed" and current["retryTicket"] != state["retryTicket"]:
                    break
                time.sleep(.02)
            else:
                self.fail("explicit retry did not finish")
            self.assertIsNotNone(current["retryTicket"])
            self.assertEqual((self.root / "app.pid").read_text(), app_pid)
            self.assertFalse(model.with_suffix(".pid").exists())
            self.assertEqual(self.sidecar.read_text(), self.original)
        finally:
            (self.root / "release-app").touch()
            out, err = process.communicate(timeout=20)
            print("Owned artifact-failure launcher diagnostics:\n" + out + err)
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("no runtime was started", err)
        self.assertNotIn("cleanup complete", (out + err).lower())
        self.assertFalse(list(self.root.glob("gemma-session-*.json*")))

    def test_pre_ready_failure_receipt_through_actual_broker(self):
        broker = launcher.ROOT / "src-tauri/target/debug/grammar-gemma-broker.exe"
        self.assertTrue(broker.is_file(), "Build the current broker first")
        shutil.copyfile(broker, self.root / broker.name)
        runtime = self.root / "local/runtime/llama-server.exe"
        subprocess.run(["rustc", str(HERE / "fixtures/gemma-startup-runtime.rs"), "-o", str(runtime)], check=True, capture_output=True)
        model = self.root / "local/gemma-4-12b-it-qat-q4_0.gguf"
        model.write_text("invalid")
        catalog_path = self.root / "Gemma-Assets.json"
        catalog = json.loads(catalog_path.read_text())
        catalog["runtimeSha256"] = launcher.digest(runtime)
        catalog["runtimeBundleSha256"] = launcher.hashlib.sha256(("llama-server.exe\0" + launcher.digest(runtime) + "\n").encode()).hexdigest()
        catalog["model"]["sha256"] = launcher.digest(model)
        catalog_path.write_text(json.dumps(catalog))
        for entry in self.manifest["files"]:
            path = self.root / entry["path"]
            entry.update(sha256=launcher.digest(path), bytes=path.stat().st_size)
        (self.root / "MANIFEST.json").write_text(json.dumps(self.manifest))
        (self.root / "wait-release").touch()
        process = subprocess.Popen(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1"), "-EnableDesktopGemma"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        try:
            session_path, state = self.wait_phase("failed")
            self.assertIsNotNone(state["retryTicket"], "actual cleanup receipt did not reach launcher")
            self.assertTrue(model.with_suffix(".warmup").exists())
            workspace = pathlib.Path(model.with_suffix(".cwd").read_text()).parent
            self.assertFalse(workspace.exists(), "workspace survived acknowledged cleanup")
            # Reuse the exact owned PID liveness assertion in tearDown.
            shutil.copyfile(model.with_suffix(".pid"), self.root / "broker.pid")
            app_pid = (self.root / "app.pid").read_text()
            old_runtime_pid = model.with_suffix(".pid").read_text()
            session_path.with_suffix(".json.retry").write_text(state["retryTicket"])
            deadline = time.monotonic() + 10
            next_state = state
            while time.monotonic() < deadline:
                next_state = json.loads(session_path.read_text())
                if next_state["phase"] == "failed" and next_state["retryTicket"] != state["retryTicket"]:
                    break
                time.sleep(.02)
            self.assertIsNotNone(next_state["retryTicket"])
            self.assertNotEqual(next_state["retryTicket"], state["retryTicket"])
            self.assertNotEqual(model.with_suffix(".pid").read_text(), old_runtime_pid)
            self.assertEqual((self.root / "app.pid").read_text(), app_pid)
            self.assertFalse(pathlib.Path(model.with_suffix(".cwd").read_text()).parent.exists())
            shutil.copyfile(model.with_suffix(".pid"), self.root / "broker.pid")
            self.assertFalse((self.root / "app-exited").exists())
            self.assertEqual(self.sidecar.read_text(), self.original)
            time.sleep(.3)
            self.assertIsNone(process.poll(), "app ownership was released")
        finally:
            (self.root / "release-app").touch()
            out, err = process.communicate(timeout=20)
        self.assertNotEqual(process.returncode, 0, "failed warmup must remain a failure")
        self.assertIn("cleanup confirmed", err)
        self.assertFalse(list(self.root.glob("gemma-session-*.json*")))


if __name__ == "__main__":
    unittest.main(defaultTest=["ProductionBrokerTests.test_pre_ready_failure_receipt_through_actual_broker", "ProductionBrokerTests.test_artifact_failure_is_retryable_without_claiming_runtime_cleanup"], verbosity=2)
