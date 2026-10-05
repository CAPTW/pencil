"""Boundary tests for the actual Python channel owner; fake native/core ports.
Not native authentication or installed product acceptance.
"""
import ctypes
import importlib.util
from pathlib import Path
from types import SimpleNamespace
import time
import unittest

path = Path(__file__).resolve().parents[2] / 'adapters/libreoffice/pythonpath/grammar_writer_bridge.py'
spec = importlib.util.spec_from_file_location('writer_bridge', path)
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)


class Fn:
    def __init__(self, fn): self.fn = fn
    def __call__(self, *args): return self.fn(*args)


class Core:
    def __init__(self):
        self.document, self.revoked, self.pending = object(), False, False
        self.result, self.calls = None, []
    def execute(self, args):
        import json
        values = {a.Name: a.Value for a in args}
        self.calls.append(values)
        action = values['Action']
        if action == 'state': result = dict(bindingRetained=self.document is not None, pending=False)
        elif action == 'poll': result, self.result = self.result or dict(status='PENDING'), None
        elif action == 'disable':
            self.document, self.revoked = None, True
            result = dict(status='DISABLED')
        else:
            self.result = (dict(status='CAPTURED', source='Owned 😀', token='review-token') if action == 'capture'
                           else dict(status='APPLIED'))
            result = dict(status='QUEUED')
        return json.dumps(result)


class Native:
    def __init__(self, commands=(1, 2), malformed=False, close_code=0):
        self.commands = list(commands)
        self.replies, self.closes = [], 0
        self.malformed, self.close_code = malformed, close_code
        self.GrammarBrowserServerVersion = Fn(lambda: 2)
        self.GrammarBrowserServerCreateV2 = Fn(self.create)
        self.GrammarBrowserServerPollV2 = Fn(self.poll)
        self.GrammarBrowserServerRespond = Fn(self.respond)
        self.GrammarBrowserServerClose = Fn(self.close)
    def create(self, cookie, anchor):
        cookie._obj.value, anchor._obj.value = 9, 1000
        return 0
    def poll(self, cookie, state, command, capture, buffer, capacity, units, ticks):
        if not self.commands:
            state._obj.value = 2
            return 0
        state._obj.value, command._obj.value = 1, self.commands.pop(0)
        ticks._obj.value, capture._obj.value = 2000, 1
        value = 'Reviewed 😀'.encode('utf-16-le')
        if command._obj.value == 2:
            ctypes.memmove(ctypes.addressof(buffer), value, len(value))
            units._obj.value = 9000 if self.malformed else len(value) // 2
        return 0
    def respond(self, cookie, status, source, units):
        self.replies.append((status, source, units))
        return 0
    def close(self, cookie):
        self.closes += 1
        return self.close_code


def owner(native):
    # Replace only the foreign-library boundary. Buffer operations remain real.
    api = {name: getattr(ctypes, name) for name in ('c_ulonglong', 'c_uint32', 'POINTER', 'c_wchar_p',
        'create_unicode_buffer', 'byref', 'memset', 'sizeof', 'addressof', 'string_at')}
    api.update(WinDLL=lambda *a, **k: native,
        windll=SimpleNamespace(kernel32=SimpleNamespace(GetTickCount64=Fn(lambda: 1000))))
    bridge.ctypes = SimpleNamespace(**api)
    return bridge.WriterBridge(Core(), 'trusted-installed-path')


class Tests(unittest.TestCase):
    def test_review_binding_utf16_and_original_deadline(self):
        native = Native()
        actual = owner(native)
        before = time.monotonic()
        actual.run()
        self.assertEqual([v[0] for v in native.replies], [2, 3])
        self.assertEqual(native.replies[0][2], len('Owned 😀'.encode('utf-16-le')) // 2)
        accepts = [v for v in actual.core.calls if v['Action'] == 'accept']
        self.assertEqual(accepts[0]['Token'], 'review-token')
        self.assertEqual(accepts[0]['Replacement'], 'Reviewed 😀')
        self.assertEqual(accepts[0]['DeadlineTicks'], 2000)
        self.assertLess(accepts[0]['Deadline'] - before, 1.1)
        self.assertTrue(actual.cleanup_complete)
        self.assertEqual(native.closes, 1)
    def test_terminal_reply_is_consumed_before_disconnect(self):
        class BufferedNative(Native):
            consumed = False
            def poll(self, *args):
                if self.replies and self.replies[-1][0] == 3:
                    self.consumed = True
                return super().poll(*args)
            def close(self, cookie):
                if not self.consumed:
                    self.close_code = 4
                return super().close(cookie)
        native = BufferedNative()
        actual = owner(native)
        actual.run()
        self.assertTrue(native.consumed, 'disconnect discarded the buffered terminal reply')
        self.assertTrue(actual.cleanup_complete)

    def test_oversized_native_text_never_reaches_core(self):
        actual = owner(Native(malformed=True))
        actual.run()
        self.assertFalse(any(v['Action'] == 'accept' for v in actual.core.calls))
        self.assertIn('native_text_bounds', actual.error)
        self.assertTrue(actual.finished.is_set())
    def test_unknown_native_command_never_reads_document(self):
        actual = owner(Native(commands=(99,)))
        actual.run()
        self.assertFalse(any(v['Action'] in ('capture', 'accept') for v in actual.core.calls))
        self.assertIn('unexpected_native_command', actual.error)
    def test_native_cleanup_failure_is_not_complete(self):
        actual = owner(Native(commands=(), close_code=2))
        actual.run()
        self.assertFalse(actual.cleanup_complete)
        self.assertEqual(actual.error, 'native_channel_cleanup_failed')
    def test_stop_revokes_before_worker_runs(self):
        actual = owner(Native())
        actual.stop()
        actual.run()
        self.assertFalse(any(v['Action'] == 'capture' for v in actual.core.calls))
        self.assertTrue(actual.core.revoked)
        self.assertTrue(actual.cleanup_complete)


if __name__ == '__main__': unittest.main()
