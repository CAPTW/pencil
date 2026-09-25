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

        def invoke(data, origin=ORIGIN):
            return subprocess.run([str(host), origin], input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=10)

        result = invoke(frame(request))
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
        deep = invoke(frame({**request, 'op': 'deep'}))
        assert json.loads(deep.stdout[4:])['error'] == 'deep_consent_required'
        assert invoke(b'').returncode == 0
        print('PASS: real host local engine/UTF16, framing, size, origin, schema, Deep missing-consent rejected, EOF; no Provider')


if __name__ == '__main__':
    main()
