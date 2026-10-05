"""Explicit Writer menu entry. No startup monitoring or public mutation API."""
import ctypes
import hashlib
import json
from pathlib import Path
import threading
import time
from types import SimpleNamespace
import unohelper
from com.sun.star.task import XJobExecutor
from com.sun.star.awt import XActionListener, XTopWindowListener, XCallback
from grammar_writer_core import WriterCore
from grammar_writer_bridge import WriterBridge
from grammar_writer_installation import LIBRARY_PATH, LIBRARY_SHA256, ALLOW_SAVED_DOCX, ALLOW_UNIFORM_FORMAT, ALLOW_MIXED_FORMAT

_owner = None


class Tick(unohelper.Base, XCallback):
    def __init__(self, owner): self.owner = owner
    def notify(self, ignored): self.owner.update()


class Panel(unohelper.Base, XActionListener, XTopWindowListener):
    def __init__(self, context):
        self.context = context
        self.core = WriterCore(context, allow_saved_documents=ALLOW_SAVED_DOCX is True,
            allow_uniform_format=ALLOW_UNIFORM_FORMAT is True, allow_mixed_format=ALLOW_MIXED_FORMAT is True)
        self.bridge = None
        self.starting = False
        self.closed = False
        self.timer = None
        self.image_handle = None
        self.doc = self.core.desktop.CurrentComponent
        self.dialog = context.ServiceManager.createInstanceWithContext('com.sun.star.awt.UnoControlDialog', context)
        model = context.ServiceManager.createInstanceWithContext('com.sun.star.awt.UnoControlDialogModel', context)
        model.Title, model.Width, model.Height = 'Grammar Writer — disabled', 235, 83
        for name, kind, label, x, y, w, h in (
            ('status', 'FixedText', ('Select text with one bold/color style in one paragraph.' if ALLOW_MIXED_FORMAT is True
                else 'Enable one uniformly bold/colored paragraph.' if ALLOW_UNIFORM_FORMAT is True
                else 'Enable this plain-body document (new or local DOCX).' if ALLOW_SAVED_DOCX is True
                else 'Enable only this new plain-text document for one review.') + '\nSelect text in Writer, then press Ctrl+Shift+G.', 8, 8, 218, 42),
            ('enable', 'Button', 'Enable one review', 8, 57, 105, 17),
            ('disable', 'Button', 'Disable', 122, 57, 105, 17)):
            control = model.createInstance('com.sun.star.awt.UnoControl' + kind + 'Model')
            control.Name, control.Label = name, label
            control.PositionX, control.PositionY, control.Width, control.Height = x, y, w, h
            model.insertByName(name, control)
        self.dialog.setModel(model)
        toolkit = context.ServiceManager.createInstanceWithContext('com.sun.star.awt.Toolkit', context)
        self.dialog.createPeer(toolkit, self.doc.CurrentController.Frame.ContainerWindow)
        for name in ('enable', 'disable'):
            self.dialog.getControl(name).setActionCommand(name)
            self.dialog.getControl(name).addActionListener(self)
        self.dialog.addTopWindowListener(self)
        self.doc.addEventListener(self)
        self.dialog.setVisible(True)

    def call(self, action, **values):
        values.update(Action=action, Deadline=time.monotonic() + 2.5)
        return json.loads(self.core.execute(tuple(SimpleNamespace(Name=k, Value=v) for k, v in values.items())))

    def status(self, text):
        if not self.closed:
            self.dialog.getControl('status').getModel().Label = text

    def pin_library(self):
        # Retain a read-only, no-write/no-delete sharing handle across load/use.
        # Installation config is generated from the paired trusted package.
        api = ctypes.windll.kernel32
        api.CreateFileW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p,
                                   ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p]
        api.CreateFileW.restype = ctypes.c_void_p
        handle = api.CreateFileW(LIBRARY_PATH, 0x80000000, 1, None, 3, 0x80, None)
        if handle in (None, ctypes.c_void_p(-1).value):
            raise RuntimeError('installed_library_unavailable')
        self.image_handle = handle
        if hashlib.sha256(Path(LIBRARY_PATH).read_bytes()).hexdigest() != LIBRARY_SHA256:
            self.release_library()
            raise RuntimeError('installed_library_changed')

    def release_library(self):
        if self.image_handle is not None:
            close = ctypes.windll.kernel32.CloseHandle
            close.argtypes, close.restype = [ctypes.c_void_p], ctypes.c_int
            close(self.image_handle)
            self.image_handle = None

    def actionPerformed(self, event):
        if event.ActionCommand == 'disable':
            self.revoke()
            return
        if event.ActionCommand != 'enable' or self.starting or (self.bridge and not self.bridge.finished.is_set()):
            return
        if self.bridge and not self.bridge.cleanup_complete:
            self.status('Cleanup is not confirmed. Close this document before trying again.')
            return
        try:
            self.pin_library()
            reply = self.call('enable', Document=self.doc)
            if reply['status'] != 'QUEUED':
                raise RuntimeError('document_not_available')
            self.starting = True
            self.dialog.getControl('enable').setEnable(False)
            self.status('Checking this document…')
            self.schedule()
        except Exception:
            self.release_library()
            self.status('Cannot enable. Use a new, unsaved plain-text Writer document and the paired Grammar installation.')

    def schedule(self):
        if self.timer is not None:
            return
        def post():
            try:
                callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
                callback.addCallback(Tick(self), None)
            except Exception:
                if self.bridge:
                    self.bridge.stop()
        self.timer = threading.Timer(.1, post)
        self.timer.daemon = True
        self.timer.start()

    def update(self):
        self.timer = None
        if self.starting:
            result = self.call('poll')
            if result['status'] == 'PENDING':
                self.schedule()
                return
            self.starting = False
            if result['status'] == 'ENABLED' and not self.closed:
                try:
                    self.bridge = WriterBridge(self.core, LIBRARY_PATH)
                    self.bridge.start()
                    self.dialog.getModel().Title = 'Grammar Writer — enabled for one review'
                    self.status('Enabled. Return to Writer, select text, and press Ctrl+Shift+G. Review and Accept in Grammar. Disable stops access.')
                    self.doc.CurrentController.Frame.ComponentWindow.setFocus()
                except Exception:
                    self.call('disable')
                    self.status('Connection unavailable. Document access has been revoked.')
            else:
                self.status('Document is unsupported or permission was revoked. No text was changed.')
        if self.bridge and not self.bridge.finished.is_set():
            self.schedule()
            return
        if self.core.pending:
            self.schedule()
            return
        if self.core.result is not None:
            self.call('poll')
        self.release_library()
        if not self.closed:
            self.dialog.getModel().Title = 'Grammar Writer — disabled'
            if self.bridge:
                self.status('Review ended. Enable again for another review.' if self.bridge.cleanup_complete else 'Cleanup is not confirmed. Close this document.')
            self.dialog.getControl('enable').setEnable(not self.bridge or self.bridge.cleanup_complete)
        else:
            self.doc = None

    def revoke(self):
        if self.bridge and not self.bridge.finished.is_set():
            self.bridge.stop()
        else:
            self.call('disable')
        self.status('Disabled. Waiting for cleanup…')
        self.schedule()

    def windowClosing(self, event):
        if self.closed:
            return
        self.closed = True
        self.revoke()
        self.doc.removeEventListener(self)
        self.dialog.setVisible(False)
        self.dialog.dispose()

    def disposing(self, event):
        if not self.closed:
            self.closed = True
            self.revoke()
            self.dialog.setVisible(False)
            self.dialog.dispose()
    def windowOpened(self, event): pass
    def windowClosed(self, event): pass
    def windowMinimized(self, event): pass
    def windowNormalized(self, event): pass
    def windowActivated(self, event): pass
    def windowDeactivated(self, event): pass


class Writer(unohelper.Base, XJobExecutor):
    def __init__(self, context): self.context = context
    def trigger(self, action):
        global _owner
        if action != 'review':
            return
        if _owner and (not _owner.closed or _owner.starting or (_owner.bridge and not _owner.bridge.finished.is_set())):
            if not _owner.closed:
                _owner.dialog.setVisible(True)
            return
        _owner = Panel(self.context)


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(Writer, 'org.grammar.Writer', ('org.grammar.Writer',))
