"""Actual host -> shared runtime -> explicit synthetic executable; never a live Provider.
Usage: python test_host_deep.py HOST_EXE FIXTURE_EXE NEW_EVIDENCE_DIRECTORY
The fixture supports P01_FIXTURE_MODE=success|slow and writes P01_CASE_DIR/root.pid.
"""
import asyncio
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import sys

ORIGIN = 'chrome-extension://' + 'a' * 32 + '/'
SOURCE = 'Synthetic writing.'

def frame(value):
    body = json.dumps(value).encode('utf-8')
    return struct.pack('<I', len(body)) + body

def exited(pid):
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = kernel.OpenProcess(0x1000, False, pid)
    if not handle:
        return ctypes.get_last_error() == 87  # ERROR_INVALID_PARAMETER: PID absent.
    try:
        code = wintypes.DWORD()
        return bool(kernel.GetExitCodeProcess(handle, ctypes.byref(code))) and code.value != 259
    finally:
        kernel.CloseHandle(handle)

async def read_response(proc):
    header = await asyncio.wait_for(proc.stdout.readexactly(4), 20)
    size, = struct.unpack('<I', header)
    assert 0 < size <= 256 * 1024
    return json.loads(await asyncio.wait_for(proc.stdout.readexactly(size), 20))

async def case(host, fixture, root, name):
    owned = root / name
    owned.mkdir()
    temporary = owned / 'temp'
    temporary.mkdir()
    # Construct a minimal environment. Do not copy any account/credential environment.
    system = os.environ.get('SystemRoot', r'C:\Windows')
    env = {'SystemRoot': system, 'WINDIR': system, 'PATH': str(Path(system) / 'System32'),
           'TEMP': str(temporary), 'TMP': str(temporary),
           'P01_CASE_DIR': str(owned), 'P01_FIXTURE_MODE': 'success' if name == 'success' else 'slow',
           'CODEX_PENCIL_CLAUDE_BIN': str(fixture), 'CODEX_PENCIL_AGY_BIN': str(fixture),
           'CODEX_PENCIL_CODEX_BIN': str(fixture)}
    proc = await asyncio.create_subprocess_exec(str(host), ORIGIN, env=env,
        stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
    request = dict(version=1, op='deep', id=name, epoch='synthetic-document', revision=4,
                   text=SOURCE, provider='claude', consent=name != 'denied')
    receipt = None
    try:
        proc.stdin.write(frame(request))
        await proc.stdin.drain()
        if name in ('cancel', 'eof'):
            async def started():
                while not ((owned / 'root.pid').exists() and (owned / 'flags.json').exists()):
                    await asyncio.sleep(.01)
            await asyncio.wait_for(started(), 15)
            assert not exited(int((owned / 'root.pid').read_text().strip()))
            if name == 'cancel':
                proc.stdin.write(frame(dict(version=1, op='cancel', id=name,
                    epoch=request['epoch'], revision=4, text='')))
                await proc.stdin.drain()
            else:
                proc.stdin.close()
        if name != 'eof':
            receipt = await read_response(proc)
            assert (receipt['id'], receipt['epoch'], receipt['revision']) == (name, request['epoch'], 4)
            if name == 'success':
                assert receipt['provider'] == 'claude' and receipt['replacement']
                assert receipt['source_sha256'] == hashlib.sha256(SOURCE.encode()).hexdigest()
                assert receipt['cleanup_complete'] is True and 'error' not in receipt
            elif name == 'denied':
                assert receipt['error'] == 'deep_consent_required'
                assert not (owned / 'root.pid').exists()
            else:
                assert receipt['error'] == 'rewrite_interrupted' and receipt['cleanup_complete'] is True
            proc.stdin.close()
        assert await asyncio.wait_for(proc.wait(), 20) == 0
        assert await proc.stderr.read() == b''
        if name != 'denied':
            assert exited(int((owned / 'root.pid').read_text().strip()))
            flags = json.loads((owned / 'flags.json').read_text(encoding='utf-8'))
            assert flags == {'no_session_persistence': True, 'bare': True,
                             'disallowed_all_tools': True, 'disable_slash_commands': True}
        runtime = temporary / 'codex-pencil-runtime-v1'
        residuals = [p.name for p in runtime.glob('client-*')]
        assert not residuals
        result = {'case': name, 'classification': 'NATIVE_SYNTHETIC_PROVIDER',
                  'host_exit': 0, 'owned_fixture_exited': name != 'denied',
                  'provider_not_admitted': name == 'denied', 'runtime_sessions_remaining': 0,
                  'claude_privacy_flags_verified': name != 'denied',
                  'cleanup_receipt': receipt.get('cleanup_complete') if receipt else None}
        (owned / 'receipt.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
        return result
    finally:
        if proc.returncode is None:
            proc.stdin.close()
            try:
                await asyncio.wait_for(proc.wait(), 20)
            except TimeoutError:
                proc.kill()  # Only the exact task-owned host created above; test remains failed.
                await proc.wait()

async def main():
    host_source, fixture, root = map(Path, sys.argv[1:4])
    host_source = host_source.resolve(strict=True)
    fixture = fixture.resolve(strict=True)
    root = root.resolve()
    root.mkdir(parents=True, exist_ok=False)
    host = root / 'grammar-chromium-host.exe'
    shutil.copyfile(host_source, host)
    (root / 'grammar-chromium-host.origin.json').write_text(json.dumps({'origin': ORIGIN}), encoding='utf-8')
    results = []
    for name in ('denied', 'success', 'cancel', 'eof'):
        results.append(await case(host, fixture, root, name))
    report = {'classification': 'NATIVE_SYNTHETIC_PROVIDER_NOT_LIVE',
              'host_sha256': hashlib.sha256(host.read_bytes()).hexdigest(),
              'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(), 'cases': results}
    (root / 'receipt.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report))

if __name__ == '__main__':
    asyncio.run(main())
