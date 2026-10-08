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
 assert_eq!(std::env::args().skip(1).count(),7);
 if dir.join("fail-start").exists() { std::process::exit(2); }
 println!(r#"{{"version":1,"pid":{},"name":"\\\\.\\pipe\\Grammar.Gemma.Broker.00000000-0000-0000-0000-000000000001"}}"#,std::process::id());
 std::io::stdout().flush().unwrap();
 let mut byte=[0]; let _=std::io::stdin().read(&mut byte);
 if !dir.join("no-cleanup").exists() { println!("{{\"cleanup\":\"complete\"}}"); }
}
'''
APP = r'''
fn main() {
 let dir=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
 std::fs::write(dir.join("app-started"),b"yes").unwrap();
 if dir.join("external-change").exists() {
   std::fs::write(dir.join("grammar-chromium-host.origin.json"),b"{\"outside\":true}").unwrap();
 }
 std::thread::sleep(std::time::Duration::from_millis(if dir.join("hold-app").exists() {3000} else {400}));
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

    def run_launcher(self):
        return subprocess.run(["pwsh", "-NoProfile", "-File", str(self.root / "Start-Grammar.ps1")], capture_output=True, text=True, encoding="utf-8", timeout=30)

    def test_ready_app_exit_revokes_and_restores(self):
        result = self.run_launcher()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Gemma cleanup complete.", result.stdout)
        self.assertTrue((self.root / "app-started").exists())
        self.assertEqual(self.sidecar.read_text(), self.original)

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

    def prepare_fixture(self, corrupt=False):
        # All paths belong to this test's newly allocated temporary directory.
        shutil.rmtree(self.root / "local")
        cache = self.root / "cache"
        cache.mkdir()
        with zipfile.ZipFile(cache / "runtime.zip", "w") as z:
            z.writestr("llama-server.exe", b"synthetic runtime never executed")
            z.writestr("llama-bench.exe", b"unneeded upstream tool")
        model = cache / "model.gguf"
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
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "broker.pid").exists())

    def test_start_failure_never_opens_app(self):
        (self.root / "fail-start").touch()
        result = self.run_launcher()
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
        result = self.run_launcher()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Host configuration changed externally", result.stderr)
        self.assertEqual(json.loads(self.sidecar.read_text()), {"outside": True})


if __name__ == "__main__":
    unittest.main(verbosity=2)

