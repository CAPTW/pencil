"""Measure pinned installed broker startup under owned_trial.py; no document requests.

Run only in a task-owned, single-session Windows trial. The installed launcher and
settings are not modified. Cache state is uncontrolled. Only fixed numeric broker
telemetry is retained; stdout pipe identity and arbitrary stderr are discarded.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import time

PHASE = re.compile(r'gemma_(?:boot|engine) phase=(?:artifact_pins|model_hash|model_verified|lease|spawn|health|warmup|ready) ms=[0-9]{1,12}')
MODEL = re.compile(r'gemma_model_pin elapsed_ms=[0-9]{1,12} read_us=[0-9]{1,16} hash_us=[0-9]{1,16} bytes=[0-9]{1,16} verified=(?:true|false)')


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--install-root', type=Path, required=True)
    parser.add_argument('--model-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if os.name != 'nt' or os.environ.get('GRAMMAR_HOST_TRIAL') != '20261011-owned':
        raise SystemExit('Owned Windows trial required')
    if args.output.exists():
        raise SystemExit('Preserve existing receipt')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.model_root = args.model_root.resolve()
    root = args.install_root.resolve()
    manifest = json.loads((root/'MANIFEST.json').read_text(encoding='utf-8-sig'))
    pins = {item['path']: item['sha256'] for item in manifest['files']}
    for name in ['grammar-gemma-broker.exe', 'grammar-chromium-host.exe', 'codex-pencil.exe', 'Gemma-Assets.json'] + [name for name in pins if name.startswith('vc-runtime/')]:
        with (root/name).open('rb') as stream:
            if hashlib.file_digest(stream, 'sha256').hexdigest() != pins[name]:
                raise SystemExit('Installed package mismatch')
    assets = json.loads((root/'Gemma-Assets.json').read_text(encoding='utf-8-sig'))
    env = dict(os.environ)
    env['PATH'] = str(root/'vc-runtime') + os.pathsep + env['PATH']
    env['GRAMMAR_GEMMA_TIMING'] = '1'
    command = [str(root/'grammar-gemma-broker.exe'), str(args.model_root/'runtime/llama-server.exe'), assets['runtimeSha256'], str(args.model_root/assets['model']['name']), assets['model']['sha256'], str(root/'grammar-chromium-host.exe'), pins['grammar-chromium-host.exe'], assets['runtimeBundleSha256'], str(root/'codex-pencil.exe'), pins['codex-pencil.exe']]
    receipt = {'status':'FAIL', 'scope':'INSTALLED_BROKER_STARTUP_ONLY_NOT_APP_ACCEPTANCE', 'sourceCommit':manifest['sourceCommit'], 'probeSha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'coldCache':'UNCONTROLLED', 'phases':[]}
    messages = queue.Queue(maxsize=16)
    started = time.monotonic()
    child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, creationflags=subprocess.CREATE_NO_WINDOW)
    def drain():
        # Bounded lines; arbitrary content is neither retained nor printed.
        for raw in iter(lambda: child.stderr.readline(2048), b''):
            line = raw.decode('ascii', errors='replace').strip()
            if len(receipt['phases']) < 32 and (PHASE.fullmatch(line) or MODEL.fullmatch(line)):
                receipt['phases'].append({'observedMs':round((time.monotonic()-started)*1000,3), 'metric':line})
    def protocol():
        for raw in iter(lambda: child.stdout.readline(2048), b''):
            try:
                obj = json.loads(raw)
                if obj.get('version') == 1 and obj.get('pid') == child.pid and isinstance(obj.get('name'), str):
                    messages.put_nowait('ready')
                elif obj == {'cleanup':'complete'}:
                    messages.put_nowait('cleanup')
            except (ValueError, queue.Full):
                pass
        try:
            messages.put_nowait('eof')
        except queue.Full:
            pass
    readers = [threading.Thread(target=drain, daemon=True), threading.Thread(target=protocol, daemon=True)]
    for reader in readers: reader.start()
    try:
        receipt['ready'] = messages.get(timeout=190) == 'ready'
        receipt['readyMs'] = round((time.monotonic()-started)*1000,3)
    except queue.Empty:
        receipt['ready'] = False
    finally:
        try:
            child.stdin.close()
        except BrokenPipeError:
            pass
        try:
            child.wait(timeout=35)
        except subprocess.TimeoutExpired:
            receipt['cleanup'] = 'UNCONFIRMED_SUPERVISOR_BACKSTOP_REQUIRED'
        for reader in readers: reader.join(timeout=2)
        pending = []
        while not messages.empty(): pending.append(messages.get_nowait())
        receipt['exitCode'] = child.poll()
        receipt['cleanupAcknowledged'] = 'cleanup' in pending
        if receipt.get('ready') and child.returncode == 0 and receipt['cleanupAcknowledged'] and all(not reader.is_alive() for reader in readers):
            receipt['status'] = 'PASS'
        args.output.write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    raise SystemExit(0 if receipt['status']=='PASS' else 1)

if __name__ == '__main__':
    main()
