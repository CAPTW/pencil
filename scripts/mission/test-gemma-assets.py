"""Exercise the production preparer with tiny owned, offline artifact archives."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
sha = lambda b: hashlib.sha256(b).hexdigest()

class Assets(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='grammar-assets-',dir=os.environ.get('GRAMMAR_ASSET_TEST_ROOT'))
        self.root = Path(self.temp.name)
        self.install = self.root/'install'; self.install.mkdir()
        self.cache = self.root/'cache'; self.cache.mkdir()
        shutil.copyfile(ROOT/'scripts/package/Prepare-GrammarLocal.ps1', self.install/'Prepare-GrammarLocal.ps1')
        self.runtime = {'llama-server.exe': b'server', 'ggml-cuda.dll': b'cuda'}
        self.entries = list(self.runtime.items()) + [('unused.exe', b'never materialize'), ('LICENSE', b'license')]
        self.model = bytes(range(256))*17000  # exercise multi-megabyte integrity checks
        self.timing = self.root/'timing.json'
    def tearDown(self):
        self.temp.cleanup()
    def setup_catalog(self):
        archive = self.cache/'runtime.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
            for name, data in self.entries: z.writestr(name,data)
        model=self.cache/'model.gguf'; model.write_bytes(self.model)
        def asset(p): return {'name':p.name,'size':p.stat().st_size,'sha256':sha(p.read_bytes())}
        bundle=''.join(f'{name}\0{sha(data)}\n' for name,data in sorted(self.runtime.items()))
        self.catalog={'schema':'grammar-local-assets/v1','runtimeArchives':[asset(archive)],'model':asset(model),'runtimeBundleSha256':sha(bundle.encode()),'runtimeSha256':sha(self.runtime['llama-server.exe'])}
        self.publish_catalog()
    def publish_catalog(self):
        p=self.install/'Gemma-Assets.json';p.write_text(json.dumps(self.catalog))
        (self.install/'MANIFEST.json').write_text(json.dumps({'sourceCommit':'owned-fixture','files':[{'path':p.name,'sha256':sha(p.read_bytes())}]}))
    def run_prepare(self, success):
        p=self.execute()
        self.assertEqual(p.returncode==0, success, p.stdout+p.stderr)
        self.assertEqual((self.install/'local').exists(), success)
        self.assertFalse((self.install/'local.pending').exists())
    def execute(self):
        sandbox=os.environ.get('GRAMMAR_ASSET_TEST_SANDBOX_ID')
        if not sandbox:
            return subprocess.run(['pwsh','-NoProfile','-File',str(self.install/'Prepare-GrammarLocal.ps1'),'-AssetCache',str(self.cache),'-TimingReceiptPath',str(self.timing)],capture_output=True,text=True,encoding='utf-8',timeout=90)
        # Explicit task-owned mapped directory; the same production script runs in Windows Sandbox.
        host=Path(os.environ['GRAMMAR_ASSET_TEST_ROOT']).resolve()
        def guest(p):return os.environ['GRAMMAR_ASSET_TEST_GUEST_ROOT'].rstrip('\\/')+'\\'+str(p.resolve().relative_to(host)).replace('/','\\')
        def quote(p):return "'"+guest(p).replace("'","''")+"'"
        wrapper=self.root/'invoke.ps1';log=self.root/'run.log';result=self.root/'exit.json'
        wrapper.write_text("$ErrorActionPreference='Stop'\ntry { & "+quote(self.install/'Prepare-GrammarLocal.ps1')+" -AssetCache "+quote(self.cache)+" -TimingReceiptPath "+quote(self.timing)+" *> "+quote(log)+"; $code=0 } catch { $_ | Out-String | Add-Content "+quote(log)+"; $code=1 }\n@{code=$code}|ConvertTo-Json|Set-Content "+quote(result),encoding='utf-8')
        command='C:\\GrammarPayload\\pwsh\\pwsh.exe -NoProfile -File "'+guest(wrapper)+'"'
        cli=subprocess.run(['wsb','exec','--id',sandbox,'--run-as','ExistingLogin','--command',command],capture_output=True,timeout=90)
        self.assertTrue(result.exists(),str(cli.stdout)+str(cli.stderr))
        return subprocess.CompletedProcess(command,json.loads(result.read_text(encoding='utf-8-sig'))['code'],log.read_text(encoding='utf-8-sig'), '')
    def test_complete_bytes_bundle_and_no_unused_executable(self):
        self.setup_catalog();self.run_prepare(True)
        self.assertEqual((self.install/'local/model.gguf').read_bytes(),self.model)
        actual={p.name:p.read_bytes() for p in (self.install/'local/runtime').iterdir()}
        self.assertEqual(actual,{**self.runtime,'LICENSE':b'license'})
        timing=json.loads((self.root/'timing.json').read_text(encoding='utf-8-sig'))
        self.assertTrue(timing['complete'])
        self.assertEqual([p['phase'] for p in timing['phases']],['runtime_copy_verify','runtime_extract','model_copy_verify','runtime_hash','complete'])
    def test_same_size_tampered_model_refused_and_stage_removed(self):
        self.setup_catalog(); (self.cache/'model.gguf').write_bytes(b'x'*len(self.model));self.run_prepare(False)
    def test_wrong_size_refused(self):
        self.setup_catalog(); (self.cache/'model.gguf').write_bytes(b'short');self.run_prepare(False)
    def test_bundle_hash_refused(self):
        self.setup_catalog();self.catalog['runtimeBundleSha256']='0'*64;self.publish_catalog();self.run_prepare(False)
    def test_traversal_in_skipped_executable_refused(self):
        self.entries.append(('../unused.exe',b'escape'));self.setup_catalog();self.run_prepare(False)
        self.assertFalse((self.install/'unused.exe').exists())
    def test_case_insensitive_duplicate_refused(self):
        self.entries.append(('GGML-CUDA.DLL',b'other'));self.setup_catalog();self.run_prepare(False)
    def test_collision_across_archives_refused(self):
        self.setup_catalog();self.catalog['runtimeArchives']*=2;self.publish_catalog();self.run_prepare(False)
    def test_existing_install_is_preserved(self):
        self.setup_catalog();(self.install/'local').mkdir();marker=self.install/'local/owned-marker';marker.write_bytes(b'preserve')
        p=self.execute()
        self.assertNotEqual(p.returncode,0);self.assertEqual(marker.read_bytes(),b'preserve')
    def test_unavailable_optional_diagnostics_do_not_fail_publication(self):
        self.setup_catalog();self.timing=self.root
        self.run_prepare(True)
        self.assertEqual((self.install/'local/model.gguf').read_bytes(),self.model)

if __name__=='__main__': unittest.main(verbosity=2)
