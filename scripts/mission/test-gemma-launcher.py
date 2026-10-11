"""Production PowerShell launcher with owned stub executables, not model/UI acceptance."""
import ctypes
import hashlib
import json
import pathlib
import shutil
import subprocess
import tempfile
import unittest
import zipfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]

BROKER = r'''
use std::io::{Read, Write};
fn main() {
 let dir=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
 std::fs::write(dir.join("broker.pid"),std::process::id().to_string()).unwrap();
 assert_eq!(std::env::args().skip(1).count(),9);
 if dir.join("wait-broker").exists() {
   for _ in 0..500 { if dir.join("release-broker").exists() {break;} std::thread::sleep(std::time::Duration::from_millis(10)); }
 }
 if dir.join("fail-start").exists() {
   if dir.join("confirmed-failure").exists() { println!("{{\"cleanup\":\"complete\"}}"); std::io::stdout().flush().unwrap(); }
   std::process::exit(2);
 }
 println!(r#"{{"version":1,"pid":{},"name":"\\\\.\\pipe\\Grammar.Gemma.Broker.00000000-0000-0000-0000-000000000001"}}"#,std::process::id());
 std::io::stdout().flush().unwrap();
 if dir.join("crash-after-app").exists() {
   std::thread::sleep(std::time::Duration::from_millis(500));
   for _ in 0..500 { if dir.join("app-started").exists() { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
   if dir.join("confirmed-failure").exists() { println!("{{\"cleanup\":\"complete\"}}"); std::io::stdout().flush().unwrap(); }
   std::process::exit(3);
 }
 let mut byte=[0]; let _=std::io::stdin().read(&mut byte);
 if !dir.join("no-cleanup").exists() { println!("{{\"cleanup\":\"complete\"}}"); }
}
'''
APP = r'''
fn main() {
 let dir=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
 std::fs::write(dir.join("app-started"),b"yes").unwrap();
 std::fs::write(dir.join("app.pid"),std::process::id().to_string()).unwrap();
 std::fs::write(dir.join("desktop-configured"), if std::env::var_os("GRAMMAR_GEMMA_SESSION").is_some() && std::env::var_os("GRAMMAR_GEMMA_LAUNCHER_PID").is_some() && std::env::var_os("GRAMMAR_GEMMA_BROKER_SHA256").is_some() {b"yes".as_slice()} else {b"no".as_slice()}).unwrap();
 if dir.join("external-change").exists() {
   std::fs::write(dir.join("grammar-chromium-host.origin.json"),b"{\"outside\":true}").unwrap();
 }
 if dir.join("wait-release").exists() {
   for _ in 0..1500 { if dir.join("release-app").exists() { break; } std::thread::sleep(std::time::Duration::from_millis(10)); }
 }
 std::thread::sleep(std::time::Duration::from_millis(if dir.join("hold-app").exists() {3000} else {400}));
 std::fs::write(dir.join("app-exited"),b"yes").unwrap();
}
'''

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class LauncherTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build = tempfile.TemporaryDirectory(prefix="grammar-launcher-build-")
        cls.binaries = pathlib.Path(cls.build.name)
        for name, source in (("broker", BROKER), ("app", APP)):
            path = cls.binaries / (name + ".rs")
            path.write_text(source)
            subprocess.run(["rustc", str(path), "-o", str(path.with_suffix(".exe"))], check=True, capture_output=True)

    @classmethod
    def tearDownClass(cls):
        cls.build.cleanup()

    def setUp(self):
        self.owned = tempfile.TemporaryDirectory(prefix="grammar-launcher-test-")
        self.root = pathlib.Path(self.owned.name)
        for name, binary in (("codex-pencil.exe", "app.exe"), ("grammar-gemma-broker.exe", "broker.exe"), ("grammar-chromium-host.exe", "app.exe")):
            shutil.copyfile(self.binaries / binary, self.root / name)
        shutil.copyfile(ROOT / "scripts/package/Start-Grammar.ps1", self.root / "Start-Grammar.ps1")
        shutil.copyfile(ROOT / "scripts/package/Uninstall-Grammar.ps1", self.root / "Uninstall-Grammar.ps1")
        (self.root / "install-receipt.json").write_text(json.dumps({"schema":"grammar-personal-install/v1","installRoot":str(self.root),"state":"installed","sourceCommit":"synthetic-not-a-build","nativeHost":None}))
        shutil.copyfile(ROOT / "scripts/package/Gemma-Assets.json", self.root / "Gemma-Assets.json")
        self.sidecar = self.root / "grammar-chromium-host.origin.json"
        self.original = json.dumps({"origin": "chrome-extension://" + "a" * 32 + "/", "installation_id": "synthetic-owned"})
        self.sidecar.write_text(self.original)
        local = self.root / "local"
        (local / "runtime").mkdir(parents=True)
        (local / "runtime/llama-server.exe").write_text("not executed")
        (local / "gemma-4-12b-it-qat-q4_0.gguf").write_text("not a real model")
        self.manifest = {"sourceCommit":"synthetic-not-a-build","extensionId": "a" * 32, "files": [{"path": p.name, "sha256": digest(p), "bytes": p.stat().st_size} for p in self.root.iterdir() if p.is_file()]}
        (self.root / "MANIFEST.json").write_text(json.dumps(self.manifest))

    def tearDown(self):
        pidfile = self.root / "broker.pid"
        if pidfile.exists():
            kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            kernel.OpenProcess.restype = ctypes.c_void_p
            kernel.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            kernel.CloseHandle.argtypes = [ctypes.c_void_p]
            handle = kernel.OpenProcess(0x100000, False, int(pidfile.read_text()))
            if handle:
                try:
                    self.assertEqual(kernel.WaitForSingleObject(handle, 0), 0, "owned broker still running")
                finally:
                    kernel.CloseHandle(handle)
        self.owned.cleanup()

    def run_launcher(self, *args):
        return subprocess.run(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1"), *args], capture_output=True, text=True, encoding="utf-8", timeout=30)

    def test_ready_app_exit_revokes_and_restores(self):
        result = self.run_launcher()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Gemma cleanup complete.", result.stdout)
        self.assertTrue((self.root / "app-started").exists())
        self.assertEqual((self.root / "desktop-configured").read_text(),"yes")
        self.assertEqual(self.sidecar.read_text(), self.original)

    def wait_phase(self, phase):
        deadline=time.monotonic()+10
        while time.monotonic()<deadline:
            files=list(self.root.glob("gemma-session-*.json"))
            if files:
                state=json.loads(files[0].read_text())
                if state["phase"]==phase: return files[0],state
            time.sleep(.02)
        self.fail("missing phase: "+phase)

    def test_explicit_retry_keeps_app_and_rejects_stale_ticket(self):
        (self.root / "crash-after-app").touch()
        (self.root / "confirmed-failure").touch()
        (self.root / "wait-release").touch()
        first=subprocess.Popen(["pwsh","-NoProfile","-File",str(self.root / "Start-Grammar.ps1"),"-EnableDesktopGemma"],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            path,state=self.wait_phase("failed")
            self.assertIsNotNone(state["retryTicket"])
            pid=(self.root / "broker.pid").read_text()
            app_pid=(self.root / "app.pid").read_text()
            request=path.with_suffix(".json.retry")
            request.write_text("0"*32)
            time.sleep(.5)
            self.assertEqual((self.root / "broker.pid").read_text(),pid)
            self.assertEqual(json.loads(path.read_text())["phase"],"failed")
            # A second failure issues a different ticket; old clicks cannot replay.
            request.write_text(state["retryTicket"])
            deadline=time.monotonic()+10
            while (self.root / "broker.pid").read_text()==pid and time.monotonic()<deadline:
                time.sleep(.02)
            _,second=self.wait_phase("failed")
            self.assertNotEqual(second["retryTicket"],state["retryTicket"])
            second_pid=(self.root / "broker.pid").read_text()
            request.write_text(state["retryTicket"])
            time.sleep(.5)
            self.assertEqual((self.root / "broker.pid").read_text(),second_pid)
            # User explicitly retries the new failure; app remains the same process.
            (self.root / "crash-after-app").unlink()
            request.write_text(second["retryTicket"])
            self.wait_phase("ready")
            self.assertNotEqual((self.root / "broker.pid").read_text(),pid)
            self.assertEqual((self.root / "app.pid").read_text(),app_pid)
            self.assertFalse(request.exists())
            self.assertFalse((self.root / "app-exited").exists())
            self.assertIsNone(json.loads(path.read_text())["retryTicket"])
        finally:
            (self.root / "release-app").touch()
            out,err=first.communicate(timeout=20)
        self.assertEqual(first.returncode,0,err)
        self.assertEqual(self.sidecar.read_text(),self.original)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))

    def test_unconfirmed_cleanup_never_offers_retry(self):
        (self.root / "crash-after-app").touch()
        (self.root / "wait-release").touch()
        first=subprocess.Popen(["pwsh","-NoProfile","-File",str(self.root / "Start-Grammar.ps1"),"-EnableDesktopGemma"],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            _,state=self.wait_phase("failed")
            self.assertIsNone(state["retryTicket"])
            self.assertFalse((self.root / "app-exited").exists())
        finally:
            (self.root / "release-app").touch()
            out,err=first.communicate(timeout=20)
        self.assertNotEqual(first.returncode,0)
        self.assertIn("cleanup was not confirmed",err)

    def test_confirmed_startup_failure_retries_without_restarting_app(self):
        (self.root / "fail-start").touch()
        (self.root / "confirmed-failure").touch()
        (self.root / "wait-release").touch()
        first=subprocess.Popen(["pwsh","-NoProfile","-File",str(self.root / "Start-Grammar.ps1"),"-EnableDesktopGemma"],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            path,state=self.wait_phase("failed")
            self.assertIsNotNone(state["retryTicket"])
            pid=(self.root / "broker.pid").read_text()
            app_pid=(self.root / "app.pid").read_text()
            time.sleep(.4)
            self.assertEqual((self.root / "broker.pid").read_text(),pid)
            self.assertEqual(self.sidecar.read_text(),self.original)
            (self.root / "fail-start").unlink()
            path.with_suffix(".json.retry").write_text(state["retryTicket"])
            self.wait_phase("ready")
            self.assertEqual((self.root / "app.pid").read_text(),app_pid)
            self.assertNotEqual((self.root / "broker.pid").read_text(),pid)
        finally:
            (self.root / "release-app").touch()
            out,err=first.communicate(timeout=20)
        self.assertEqual(first.returncode,0,err)
        self.assertFalse(list(self.root.glob("gemma-session-*.json*")))
        self.assertEqual(self.sidecar.read_text(),self.original)

    def test_desktop_configuration_can_be_explicitly_disabled(self):
        result = self.run_launcher("-EnableDesktopGemma:$false")
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual((self.root / "desktop-configured").read_text(),"no")
        self.assertEqual(self.sidecar.read_text(), self.original)

    def test_broker_crash_preserves_app_and_lock_until_explicit_exit_then_relaunch(self):
        (self.root / "crash-after-app").touch()
        (self.root / "wait-release").touch()
        first = subprocess.Popen(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1")], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        try:
            deadline = time.monotonic() + 10
            while not (self.root / "app-started").exists() and time.monotonic() < deadline:
                time.sleep(.02)
            self.assertTrue((self.root / "app-started").exists())
            self.wait_phase("failed")
            self.assertIsNone(first.poll(), "launcher released ownership while app still open")
            self.assertFalse((self.root / "app-exited").exists())
            self.assertEqual(self.sidecar.read_text(), self.original)
            second = self.run_launcher()
            self.assertNotEqual(second.returncode, 0)
            removal = subprocess.run(["pwsh","-NoProfile","-File",str(self.root / "Uninstall-Grammar.ps1")],capture_output=True,text=True,encoding="utf-8",timeout=10)
            self.assertNotEqual(removal.returncode,0)
            self.assertIn("wait for its Gemma launcher",removal.stderr)
        finally:
            (self.root / "release-app").touch()
            out, err = first.communicate(timeout=20)
            deadline=time.monotonic()+5
            while not (self.root / "app-exited").exists() and time.monotonic()<deadline:
                time.sleep(.02)
        self.assertNotEqual(first.returncode,0)
        self.assertIn("Close Grammar from the tray",out)
        self.assertTrue((self.root / "app-exited").exists())
        (self.root / "crash-after-app").unlink()
        retry=self.run_launcher()
        self.assertEqual(retry.returncode,0,retry.stderr)
        self.assertEqual(self.sidecar.read_text(),self.original)

    def test_confirmed_cleanup_does_not_turn_broker_failure_into_success(self):
        (self.root / "crash-after-app").touch()
        (self.root / "confirmed-failure").touch()
        result=self.run_launcher()
        self.assertNotEqual(result.returncode,0)
        self.assertIn("cleanup confirmed",result.stderr)
        self.assertNotIn("cleanup was not confirmed",result.stderr)
        self.assertTrue((self.root / "app-exited").exists())
        self.assertEqual(self.sidecar.read_text(),self.original)

    def test_desktop_opens_while_preparing_and_publishes_ready_without_relaunch(self):
        (self.root / "wait-broker").touch()
        (self.root / "wait-release").touch()
        first=subprocess.Popen(["pwsh","-NoProfile","-File",str(self.root / "Start-Grammar.ps1"),"-EnableDesktopGemma"],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            deadline=time.monotonic()+10
            while not (self.root / "app-started").exists() and time.monotonic()<deadline: time.sleep(.02)
            self.assertTrue((self.root / "app-started").exists())
            files=list(self.root.glob("gemma-session-*.json")); self.assertEqual(len(files),1)
            self.assertEqual(json.loads(files[0].read_text())["phase"],"starting")
            self.assertEqual(self.sidecar.read_text(),self.original)
            (self.root / "release-broker").touch()
            while json.loads(files[0].read_text())["phase"]!="ready" and time.monotonic()<deadline: time.sleep(.02)
            ready=json.loads(files[0].read_text());self.assertEqual(ready["phase"],"ready")
            self.assertTrue(ready["pipe"].startswith(r"\\.\pipe\Grammar.Gemma.Broker."))
            self.assertIsNone(first.poll())
            self.assertFalse((self.root / "app-exited").exists())
        finally:
            (self.root / "release-broker").touch();(self.root / "release-app").touch()
            out,err=first.communicate(timeout=20)
        self.assertEqual(first.returncode,0,err)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))
        self.assertEqual(self.sidecar.read_text(),self.original)

    def test_desktop_start_failure_preserves_app_until_explicit_exit(self):
        (self.root / "fail-start").touch();(self.root / "wait-release").touch()
        first=subprocess.Popen(["pwsh","-NoProfile","-File",str(self.root / "Start-Grammar.ps1"),"-EnableDesktopGemma"],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            deadline=time.monotonic()+10
            phase=None
            while time.monotonic()<deadline:
                files=list(self.root.glob("gemma-session-*.json"))
                if files: phase=json.loads(files[0].read_text())["phase"]
                if phase=="failed": break
                time.sleep(.02)
            self.assertEqual(phase,"failed")
            self.assertIsNone(json.loads(files[0].read_text())["retryTicket"])
            self.assertTrue((self.root / "app-started").exists())
            self.assertIsNone(first.poll())
            self.assertNotEqual(self.run_launcher().returncode,0)
            self.assertEqual(self.sidecar.read_text(),self.original)
        finally:
            (self.root / "release-app").touch();out,err=first.communicate(timeout=20)
        self.assertNotEqual(first.returncode,0)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))

    def test_desktop_exit_during_startup_does_not_publish_endpoint(self):
        (self.root / "wait-broker").touch()
        result=self.run_launcher("-EnableDesktopGemma")
        self.assertNotEqual(result.returncode,0)
        self.assertIn("App closed while Gemma was preparing",result.stderr)
        self.assertTrue((self.root / "app-exited").exists())
        self.assertEqual(self.sidecar.read_text(),self.original)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))

    def test_missing_cleanup_receipt_is_failure(self):
        (self.root / "no-cleanup").touch()
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cleanup was not confirmed", result.stderr)
        self.assertEqual(self.sidecar.read_text(), self.original)

    def test_concurrent_launcher_refused(self):
        (self.root / "hold-app").touch()
        first = subprocess.Popen(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1")], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        try:
            deadline = time.monotonic() + 10
            while not (self.root / "app-started").exists() and time.monotonic() < deadline:
                time.sleep(.02)
            self.assertTrue((self.root / "app-started").exists())
            second = self.run_launcher()
            self.assertNotEqual(second.returncode, 0)
            removal = subprocess.run(["pwsh","-NoProfile","-File",str(self.root / "Uninstall-Grammar.ps1")],capture_output=True,text=True,encoding="utf-8",timeout=10)
            self.assertNotEqual(removal.returncode,0)
            self.assertIn("wait for its Gemma launcher",removal.stderr)
            self.assertTrue((self.root / "codex-pencil.exe").exists())
        finally:
            out, err = first.communicate(timeout=20)
        self.assertEqual(first.returncode, 0, err)
        self.assertEqual(self.sidecar.read_text(), self.original)

    def prepare_fixture(self, corrupt=False, run=True):
        # All paths belong to this test's newly allocated temporary directory.
        shutil.rmtree(self.root / "local")
        cache = self.root / "cache"
        cache.mkdir()
        with zipfile.ZipFile(cache / "runtime.zip", "w") as z:
            z.writestr("llama-server.exe", b"synthetic runtime never executed")
            z.writestr("llama-bench.exe", b"unneeded upstream tool")
        model = cache / "gemma-4-12b-it-qat-q4_0.gguf"
        model.write_bytes(b"synthetic model never executed")
        runtime_hash = hashlib.sha256(b"synthetic runtime never executed").hexdigest()
        bundle_hash = hashlib.sha256(("llama-server.exe\0" + runtime_hash + "\n").encode()).hexdigest()
        descriptor = lambda p: {"name":p.name,"url":"https://github.com/unused", "sha256":digest(p),"size":p.stat().st_size}
        catalog = {"schema":"grammar-local-assets/v1","runtimeSha256":runtime_hash,"runtimeBundleSha256":bundle_hash,"runtimeArchives":[descriptor(cache / "runtime.zip")],"model":descriptor(model)}
        path = self.root / "Gemma-Assets.json"
        path.write_text(json.dumps(catalog))
        self.manifest["sourceCommit"] = "synthetic-not-a-build"
        for item in self.manifest["files"]:
            if item["path"] == path.name:
                item.update(sha256=digest(path), bytes=path.stat().st_size)
        (self.root / "MANIFEST.json").write_text(json.dumps(self.manifest))
        shutil.copyfile(ROOT / "scripts/package/Prepare-GrammarLocal.ps1", self.root / "Prepare-GrammarLocal.ps1")
        if corrupt:
            model.write_bytes(b"corrupted")
        if not run:
            return cache
        return self.run_prepare(cache)

    def run_prepare(self, cache):
        return subprocess.run(["pwsh", "-NoProfile", "-File",str(self.root / "Prepare-GrammarLocal.ps1"),"-AssetCache",str(cache)],capture_output=True,text=True,encoding="utf-8",timeout=30)

    def test_offline_asset_preparation(self):
        result = self.prepare_fixture()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.root / "local/assets-receipt.json").exists())
        self.assertFalse((self.root / "local/runtime/llama-bench.exe").exists())
        self.assertFalse((self.root / "local.pending").exists())
        self.assertFalse((self.root / "broker.pid").exists())

    def test_corrupt_cache_leaves_no_ready_assets(self):
        result = self.prepare_fixture(corrupt=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Asset integrity failed",result.stderr)
        self.assertFalse((self.root / "local").exists())
        self.assertFalse((self.root / "local.pending").exists())

    def test_uninstalling_installation_never_starts(self):
        path = self.root / "install-receipt.json"
        data = json.loads(path.read_text())
        data["state"] = "uninstalling"
        path.write_text(json.dumps(data))
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Installation is incomplete", result.stderr)
        self.assertFalse((self.root / "broker.pid").exists())

    def test_owned_uninstall_without_active_launcher(self):
        result = subprocess.run(["pwsh","-NoProfile","-File",str(self.root / "Uninstall-Grammar.ps1"),"-UserDataDirectory",str(self.root / "owned-unused-data")],capture_output=True,text=True,encoding="utf-8",timeout=30)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertTrue(json.loads(result.stdout)["complete"])
        self.assertFalse(self.root.exists())

    def test_bad_broker_hash_never_starts(self):
        with (self.root / "grammar-gemma-broker.exe").open("ab") as f:
            f.write(b"changed")
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Package file changed", result.stderr)
        self.assertFalse((self.root / "broker.pid").exists())

    def test_missing_model_never_starts(self):
        (self.root / "local/gemma-4-12b-it-qat-q4_0.gguf").unlink()
        result = self.run_launcher("-EnableDesktopGemma:$false")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "broker.pid").exists())

    def test_managed_missing_assets_keep_app_and_allow_explicit_retry(self):
        cache = self.prepare_fixture(run=False)
        (self.root / "wait-release").touch()
        process = subprocess.Popen(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1")], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8")
        try:
            path, state = self.wait_phase("failed")
            self.assertTrue((self.root / "app-started").exists())
            app_pid = (self.root / "app.pid").read_text()
            self.assertFalse((self.root / "broker.pid").exists())
            self.assertIsNotNone(state["retryTicket"])
            self.assertEqual(self.sidecar.read_text(), self.original)
            # A premature click fails again without allocating a broker.
            path.with_suffix(".json.retry").write_text(state["retryTicket"])
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                current = json.loads(path.read_text())
                if current["phase"] == "failed" and current["retryTicket"] != state["retryTicket"]:
                    break
                time.sleep(.02)
            else:
                self.fail("missing-assets retry did not issue a fresh ticket")
            self.assertFalse((self.root / "broker.pid").exists())
            # Preparing files alone does not start inference.
            prepared = self.run_prepare(cache)
            self.assertEqual(prepared.returncode, 0, prepared.stderr)
            self.assertTrue((self.root / "local/assets-receipt.json").exists())
            time.sleep(.3)
            self.assertFalse((self.root / "broker.pid").exists())
            path.with_suffix(".json.retry").write_text(current["retryTicket"])
            self.wait_phase("ready")
            self.assertEqual((self.root / "app.pid").read_text(), app_pid)
        finally:
            (self.root / "release-app").touch()
            out, err = process.communicate(timeout=25)
        self.assertEqual(process.returncode, 0, out + err)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))
        self.assertEqual(self.sidecar.read_text(), self.original)

    def test_managed_missing_assets_exit_reports_unavailable_and_releases_session(self):
        (self.root / "local/gemma-4-12b-it-qat-q4_0.gguf").unlink()
        result = self.run_launcher("-EnableDesktopGemma")
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.root / "app-started").exists())
        self.assertFalse((self.root / "broker.pid").exists())
        self.assertIn("no runtime was started", result.stderr)
        self.assertNotIn("Gemma cleanup complete.", result.stdout)
        self.assertFalse(list(self.root.glob("gemma-session-*.json")))
        self.assertEqual(self.sidecar.read_text(), self.original)
        # Exclusive ownership was released even on this failure path.
        result = self.run_launcher("-EnableDesktopGemma")
        self.assertIn("no runtime was started", result.stderr)

    def test_managed_missing_assets_do_not_bypass_package_integrity(self):
        (self.root / "local/gemma-4-12b-it-qat-q4_0.gguf").unlink()
        with (self.root / "grammar-gemma-broker.exe").open("ab") as f:
            f.write(b"changed")
        result = self.run_launcher("-EnableDesktopGemma")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Package file changed", result.stderr)
        self.assertFalse((self.root / "app-started").exists())
        self.assertFalse((self.root / "broker.pid").exists())

    def test_start_failure_never_opens_app(self):
        (self.root / "fail-start").touch()
        result = self.run_launcher("-EnableDesktopGemma:$false")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "app-started").exists())
        self.assertEqual(self.sidecar.read_text(), self.original)

    def test_existing_endpoint_preserved(self):
        original = json.loads(self.original)
        original["gemma"] = {"pipe": "existing"}
        value = json.dumps(original)
        self.sidecar.write_text(value)
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.sidecar.read_text(), value)
        self.assertFalse((self.root / "broker.pid").exists())

    def test_external_config_change_preserved_broker_closed(self):
        (self.root / "external-change").touch()
        result = self.run_launcher("-EnableDesktopGemma:$false")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Host configuration changed externally", result.stderr)
        self.assertEqual(json.loads(self.sidecar.read_text()), {"outside": True})


if __name__ == "__main__":
    unittest.main(verbosity=2)

