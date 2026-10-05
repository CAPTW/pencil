"""Internal Writer native-channel owner. Not an auto-start or public UNO service.

The deployment entry point must supply the pinned installed DLL path and enable
one document visibly before constructing this owner. Only the native DLL admits
peer processes. No socket/UNO argument may supply an alternative trust path.
"""
import ctypes
import json
import threading
import time
from types import SimpleNamespace


class WriterBridge:
    def __init__(self, core, installed_library):
        self.core = core
        self.library = ctypes.WinDLL(str(installed_library), winmode=0x1100)
        self.stop_requested = threading.Event()
        self.finished = threading.Event()
        self.error = None
        self.cleanup_complete = False
        self.cleanup_deadline = None
        self.thread = None
        self.cookie = ctypes.c_ulonglong()
        self.tick = ctypes.windll.kernel32.GetTickCount64
        self.tick.restype = ctypes.c_ulonglong
        u32, u64 = ctypes.c_uint32, ctypes.c_ulonglong
        ref = ctypes.POINTER
        self.create = self.library.GrammarWriterServerCreateV2
        self.create.argtypes, self.create.restype = [ctypes.c_void_p, ref(u64), ref(u64)], u32
        self.poll = self.library.GrammarBrowserServerPollV2
        self.poll.argtypes = [u64, ref(u32), ref(u32), ref(u64), ctypes.c_wchar_p, u32, ref(u32), ref(u64)]
        self.poll.restype = u32
        self.respond = self.library.GrammarBrowserServerRespond
        self.respond.argtypes, self.respond.restype = [u64, u32, ctypes.c_wchar_p, u32], u32
        self.close = self.library.GrammarBrowserServerClose
        self.close.argtypes, self.close.restype = [u64], u32
        version = self.library.GrammarBrowserServerVersion
        version.argtypes, version.restype = [], u32
        if version() != 2:
            raise RuntimeError('incompatible_native_channel')

    def call(self, action, **values):
        values['Action'] = action
        return json.loads(self.core.execute(tuple(SimpleNamespace(Name=k, Value=v) for k, v in values.items())))

    def start(self):
        if self.thread is not None or not self.core.document or self.core.revoked:
            raise RuntimeError('document_not_enabled_or_already_started')
        self.thread = threading.Thread(target=self.run, name='Grammar Writer channel', daemon=False)
        self.thread.start()

    def stop(self):
        if self.cleanup_deadline is None:
            self.cleanup_deadline = time.monotonic() + 3.5
        self.stop_requested.set()
        # Immediate revocation does not imply the native owner has finished.
        return self.call('disable', Deadline=time.monotonic() + 2.5)

    def run(self):
        token = None
        pending = None
        terminal = False
        buffer = ctypes.create_unicode_buffer(8193)
        try:
            anchor = ctypes.c_ulonglong()
            if self.create(self.core.window, ctypes.byref(self.cookie), ctypes.byref(anchor)):
                raise RuntimeError('native_channel_unavailable')
            while not self.stop_requested.is_set():
                state, command, units = ctypes.c_uint32(), ctypes.c_uint32(), ctypes.c_uint32()
                capture, ticks = ctypes.c_ulonglong(), ctypes.c_ulonglong()
                code = self.poll(self.cookie, ctypes.byref(state), ctypes.byref(command), ctypes.byref(capture),
                    buffer, len(buffer), ctypes.byref(units), ctypes.byref(ticks))
                if code or state.value == 2:
                    break
                if state.value == 1:
                    if terminal or pending is not None or command.value not in (1, 2):
                        raise RuntimeError('unexpected_native_command')
                    # Preserve the original wire deadline at every core check.
                    # Float conversion is conservative; raw ticks are also checked.
                    deadline = time.monotonic() + (ticks.value - self.tick()) / 1000 - .016
                    values = dict(Deadline=deadline, DeadlineTicks=ticks.value)
                    action = 'capture' if command.value == 1 else 'accept'
                    if action == 'accept':
                        if units.value > 8192:
                            raise RuntimeError('native_text_bounds')
                        replacement = ctypes.string_at(ctypes.addressof(buffer), units.value * 2).decode('utf-16-le')
                        values.update(Token=token, Replacement=replacement)
                        replacement = None
                        token = None
                    ctypes.memset(ctypes.addressof(buffer), 0, ctypes.sizeof(buffer))
                    pending = self.call(action, **values)
                    values = None
                if pending is not None:
                    if pending['status'] in ('QUEUED', 'PENDING'):
                        pending = self.call('poll')
                    if pending['status'] != 'PENDING':
                        status = pending['status']
                        source = pending.get('source', '') if status == 'CAPTURED' else ''
                        token = pending.get('token') if status == 'CAPTURED' else token
                        result_code = {'CAPTURED': 2, 'APPLIED': 3, 'UNKNOWN_CHANGED': 4}.get(status, 0)
                        units_out = len(source.encode('utf-16-le')) // 2
                        reply_code = self.respond(self.cookie, result_code, source, units_out)
                        pending = None
                        source = None
                        if reply_code:
                            raise RuntimeError('native_response_failed')
                        # DisconnectNamedPipe discards unread reply bytes. Keep
                        # polling until the peer closes or the native terminal
                        # deadline expires; never flush synchronously or retry.
                        terminal = result_code != 2
                if self.core.document is None or self.core.revoked:
                    break
                self.stop_requested.wait(.01)
        except Exception as error:
            self.error = type(error).__name__ + ': ' + str(error)
        finally:
            token = pending = None
            ctypes.memset(ctypes.addressof(buffer), 0, ctypes.sizeof(buffer))
            # Transport completion and target detachment are separate evidence.
            # An explicit stop owns one cleanup deadline, including a native poll
            # already in progress; no fresh budget is minted per cleanup stage.
            if self.cleanup_deadline is None:
                self.cleanup_deadline = time.monotonic() + 3.5
            try:
                self.call('disable', Deadline=min(self.cleanup_deadline, time.monotonic() + 2.5))
            except Exception:
                self.error = 'target_revocation_failed'
            finally:
                try:
                    if self.cookie.value and self.close(self.cookie):
                        self.error = 'native_channel_cleanup_failed'
                except Exception:
                    self.error = 'native_channel_cleanup_failed'
            try:
                while time.monotonic() < self.cleanup_deadline:
                    state = self.call('state')
                    if not state['bindingRetained'] and not state['pending']:
                        # Release any terminal result retained by the internal core.
                        self.call('poll')
                        self.cleanup_complete = self.error != 'native_channel_cleanup_failed'
                        break
                    time.sleep(.01)
                if not self.cleanup_complete and self.error is None:
                    self.error = 'target_cleanup_pending'
            except Exception:
                self.error = 'target_cleanup_unconfirmed'
            finally:
                self.finished.set()
