"""Synthetic executable protocol acceptance, no browser, registry, account, or Provider."""
import json
import pathlib
import shutil
import struct
import subprocess
import sys
import tempfile

ORIGIN = 'chrome-extension://' + 'a' * 32 + '/'


def frame(value):
    body = json.dumps(value, ensure_ascii=False).encode('utf-8')
    return struct.pack('<I', len(body)) + body


def main():
    executable = pathlib.Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='grammar-host-synthetic-') as temporary:
        root = pathlib.Path(temporary)
        host = root / executable.name
        shutil.copyfile(executable, host)
        (root / 'grammar-chromium-host.origin.json').write_text(json.dumps({'origin': ORIGIN}), encoding='utf-8')
        request = dict(version=1, op='analyze', id='synthetic', epoch='doc', revision=2, text='😀 seperate')

        def invoke(data, origin=ORIGIN, expect_reply=False):
            if expect_reply:
                # EOF now revokes pending async analysis. A browser keeps stdin
                # open until it has consumed the framed response.
                import concurrent.futures
                process = subprocess.Popen([str(host), origin], stdin=subprocess.PIPE,
                                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                pool = concurrent.futures.ThreadPoolExecutor(max_workers=1)
                try:
                    process.stdin.write(data)
                    process.stdin.flush()
                    def read_reply():
                        header = process.stdout.read(4)
                        assert len(header) == 4
                        size, = struct.unpack('<I', header)
                        assert 0 < size <= 256 * 1024
                        body = process.stdout.read(size)
                        assert len(body) == size
                        return header + body
                    output = pool.submit(read_reply).result(timeout=10)
                    process.stdin.close()
                    code = process.wait(timeout=10)
                    return subprocess.CompletedProcess(process.args, code, output, process.stderr.read())
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)
                    pool.shutdown(wait=True)
                    for stream in (process.stdin, process.stdout, process.stderr):
                        stream.close()
            return subprocess.run([str(host), origin], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)

        result = invoke(frame(request), expect_reply=True)
        assert result.returncode == 0 and not result.stderr
        size, = struct.unpack('<I', result.stdout[:4])
        assert size == len(result.stdout) - 4
        response = json.loads(result.stdout[4:])
        assert response['id'] == 'synthetic' and response['revision'] == 2
        assert response['suggestions']
        changed = request['text'].encode('utf-16-le')
        for suggestion in reversed(response['suggestions']):
            changed = changed[:suggestion['start'] * 2] + suggestion['replacement'].encode('utf-16-le') + changed[suggestion['end'] * 2:]
        assert changed.decode('utf-16-le') == '😀 separate'
        for data in [b'\x01', b'\x05\0\0\0x', struct.pack('<I', 128 * 1024 + 1), frame({**request, 'text': 'x' * 8193}), frame({**request, 'exec': 'denied'})]:
            rejected = invoke(data)
            assert rejected.returncode != 0 and not rejected.stdout and not rejected.stderr
        rejected = invoke(frame(request), ORIGIN.replace('aaaa', 'bbbb', 1))
        assert rejected.returncode != 0 and not rejected.stdout and not rejected.stderr
        deep = invoke(frame({**request, 'op': 'deep'}), expect_reply=True)
        assert json.loads(deep.stdout[4:])['error'] == 'deep_consent_required'
        assert invoke(b'').returncode == 0
        # Explicit Gemma selection must not silently revert to rules if its
        # launcher-provided executable pin is unavailable.
        (root / 'grammar-chromium-host.origin.json').write_text(json.dumps({
            'origin': ORIGIN, 'gemma': {
                'pipe': r'\\.\pipe\Grammar.Gemma.Broker.11111111-1111-4111-8111-111111111111',
                'broker_sha256': '0' * 64,
            }}), encoding='utf-8')
        rejected = invoke(frame(request))
        assert rejected.returncode != 0 and not rejected.stdout and not rejected.stderr
        deep = invoke(frame({**request, 'op': 'deep'}), expect_reply=True)
        assert json.loads(deep.stdout[4:])['error'] == 'deep_consent_required'
        print('PASS: real host local engine/UTF16, framing, size, origin, schema, Deep missing-consent rejected, EOF; no Provider')


if __name__ == '__main__':
    main()
