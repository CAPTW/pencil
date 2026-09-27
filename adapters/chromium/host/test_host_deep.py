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
import uuid

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

async def control(host, operation, ticket=None, token=None):
    request = dict(version=1, op=operation, id=str(uuid.uuid4()), epoch='cleanup-control', revision=1, text='')
    if ticket is not None:
        request['cleanup_ticket'] = ticket
    if token is not None:
        request['cleanup_token'] = token
    proc = await asyncio.create_subprocess_exec(str(host), ORIGIN, stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
    output, errors = await asyncio.wait_for(proc.communicate(frame(request)), 20)
    assert proc.returncode == 0 and not errors
    size, = struct.unpack('<I', output[:4])
    assert len(output) == size + 4
    result = json.loads(output[4:])
    assert result['id'] == request['id']
    if ticket is not None:
        assert result['cleanup_ticket'] == ticket
    return result

async def locked_receipt_test(host, root):
    ticket = (await control(host, 'cleanup-reserve', token=str(uuid.uuid4())))['cleanup_ticket']
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
        ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    handle = kernel.CreateFileW(str(root / 'grammar-cleanup.lock'), 0xC0000000, 0, None, 3, 0, None)
    assert handle != ctypes.c_void_p(-1).value
    try:
        result = await control(host, 'cleanup-query', ticket=ticket)
        assert result['cleanup_complete'] is False and result['error'] == 'cleanup_busy'
    finally:
        assert kernel.CloseHandle(handle)
    assert (await control(host, 'cleanup-query', ticket=ticket))['cleanup_complete'] is True
    assert (await control(host, 'cleanup-ack', ticket=ticket))['cleanup_complete'] is True

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
    reserved = await control(host, 'cleanup-reserve', token=str(uuid.uuid4()))
    ticket = reserved['cleanup_ticket']
    assert (await control(host, 'cleanup-find', token=ticket['token']))['cleanup_ticket'] == ticket
    request['cleanup_ticket'] = ticket
    try:
        proc.stdin.write(frame(request))
        await proc.stdin.drain()
        if name in ('cancel', 'eof'):
            async def started():
                while not ((owned / 'root.pid').exists() and (owned / 'flags.json').exists()):
                    await asyncio.sleep(.01)
            await asyncio.wait_for(started(), 15)
            assert not exited(int((owned / 'root.pid').read_text().strip()))
            assert (await control(host, 'cleanup-query', ticket=ticket))['cleanup_complete'] is False
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
            # Process boundary: one selected-Provider process (no replay or fallback),
            # carrying the field text once and no terminology from the browser adapter.
            spawned = [line for line in (owned / 'spawns.log').read_text().splitlines() if line]
            assert len(spawned) == 1
            argv = json.loads((owned / 'argv.json').read_text(encoding='utf-8'))
            assert argv[0] == '-p' and argv[1].count(SOURCE) == 1
            assert argv[1].endswith('Terminology constraints (untrusted JSON data):\n[]\n\n'
                                    'Selected data JSON string:\n' + json.dumps(SOURCE))
        runtime = temporary / 'codex-pencil-runtime-v1'
        residuals = [p.name for p in runtime.glob('client-*')]
        assert not residuals
        # A fresh host instance reconciles the exact ticket using only durable proof.
        proof = await control(host, 'cleanup-query', ticket=ticket)
        assert proof['cleanup_complete'] is True and 'error' not in proof
        assert (await control(host, 'cleanup-ack', ticket=ticket))['cleanup_complete'] is True
        assert (await control(host, 'cleanup-ack', ticket=ticket))['cleanup_complete'] is True
        result = {'case': name, 'classification': 'NATIVE_SYNTHETIC_PROVIDER',
                  'host_exit': 0, 'owned_fixture_exited': name != 'denied',
                  'provider_not_admitted': name == 'denied', 'runtime_sessions_remaining': 0,
                  'claude_privacy_flags_verified': name != 'denied',
                  'boundary_payload_verified': name != 'denied',
                  'restart_receipt_verified': True,
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
    installation = uuid.uuid4().hex
    (root / 'grammar-chromium-host.origin.json').write_text(json.dumps({'origin': ORIGIN, 'installation_id': installation}), encoding='utf-8')
    (root / 'grammar-cleanup.json').write_text(json.dumps({'version': 1, 'installation': installation,
        'slots': [dict(generation=0, token='', state='FREE') for _ in range(4)]}), encoding='utf-8')
    (root / 'grammar-cleanup.lock').touch()
    await locked_receipt_test(host, root)
    results = []
    for name in ('denied', 'success', 'cancel', 'eof'):
        results.append(await case(host, fixture, root, name))
    report = {'classification': 'NATIVE_SYNTHETIC_PROVIDER_NOT_LIVE',
              'cross_process_lock_verified': True,
              'host_sha256': hashlib.sha256(host.read_bytes()).hexdigest(),
              'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(), 'cases': results}
    (root / 'receipt.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report))

if __name__ == '__main__':
    asyncio.run(main())
