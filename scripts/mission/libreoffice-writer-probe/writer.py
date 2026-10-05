"""Experimental target-side Writer transaction, restricted to synthetic Sandbox.

Not product IPC, not a general-document privacy qualification, not shipped.
"""
import ctypes
import json
import math
import os
import secrets
import threading
import time
import unohelper
from com.sun.star.task import XJob
from com.sun.star.awt import XCallback
from com.sun.star.util import XModifyListener
from com.sun.star.view import XSelectionChangeListener
from com.sun.star.lang.SystemDependent import SYSTEM_WIN32


class Refused(Exception):
    pass


class Changes(unohelper.Base, XModifyListener, XSelectionChangeListener):
    def __init__(self, owner):
        self.owner = owner

    def modified(self, event):
        if self.owner:
            self.owner.revision += 1

    def selectionChanged(self, event):
        if self.owner:
            self.owner.selection_revision += 1

    def disposing(self, event):
        if self.owner:
            self.owner.clear_review()
            self.owner.document = self.owner.controller = self.owner.listener = None
            self.owner = None


class Expiry(unohelper.Base, XCallback):
    def __init__(self, owner, token):
        self.owner, self.token = owner, token

    def notify(self, ignored):
        owner = self.owner
        if owner.review and owner.review['token'] == self.token and time.monotonic() >= owner.review['expires']:
            owner.clear_review()
            if owner.result and owner.result.get('token') == self.token:
                owner.result = dict(status='EXPIRED')


class Callback(unohelper.Base, XCallback):
    def __init__(self, owner, args, deadline, epoch):
        self.owner, self.args, self.deadline, self.epoch = owner, args, deadline, epoch

    def notify(self, ignored):
        owner = self.owner
        wrote = False
        try:
            owner.deadline(self.deadline)
            action = self.args['Action']
            if action != 'disable' and owner.epoch != self.epoch:
                raise Refused('permission_revoked')
            if action == 'enable':
                owner.disable()
                document = self.args['Document']
                owner.check_document(document)
                owner.document = document
                owner.controller = document.CurrentController
                owner.listener = Changes(owner)
                document.addModifyListener(owner.listener)
                owner.controller.addSelectionChangeListener(owner.listener)
                if owner.epoch != self.epoch:
                    raise Refused('permission_revoked')
                owner.revoked = False
                value = dict(status='ENABLED')
            elif action == 'disable':
                owner.disable()
                value = dict(status='DISABLED')
            elif action == 'dismiss':
                owner.clear_review()
                value = dict(status='DISMISSED')
            elif action == 'capture':
                owner.clear_review()
                owner.permission(self.epoch)
                owner.check_document(owner.document)
                selected = owner.selected()
                source = owner.read_range(selected)
                if not source or len(source) > 4096:
                    raise Refused('source_bounds')
                owner.deadline(self.deadline)
                owner.permission(self.epoch)
                owner.review = dict(token=secrets.token_hex(24), source=source,
                    range=owner.document.Text.createTextCursorByRange(selected),
                    revision=owner.revision, selection_revision=owner.selection_revision,
                    expires=time.monotonic() + 30, epoch=self.epoch)
                owner.schedule_expiry(owner.review['token'])
                value = dict(status='CAPTURED', token=owner.review['token'], source=source)
            elif action == 'accept':
                review = owner.review
                # Consume authority before any callback-producing method or attempted write.
                owner.clear_review()
                if not review or not secrets.compare_digest(str(self.args.get('Token', '')), review['token']):
                    raise Refused('token')
                replacement = self.args.get('Replacement')
                if not isinstance(replacement, str) or not replacement or len(replacement) > 4096:
                    raise Refused('replacement_bounds')
                owner.validate(review, self.deadline)
                undo = owner.document.getUndoManager()
                undo.enterUndoContext('Grammar experimental reviewed replacement')
                try:
                    # enterUndoContext can call listeners. Recheck everything after it.
                    owner.validate(review, self.deadline)
                    # Writer's setString performs delete then insert. Defer modified
                    # notifications across that pair; otherwise a listener can move
                    # the range after delete and cause insertion into another range.
                    if not owner.document.isSetModifiedEnabled():
                        raise Refused('modification_state_owned_elsewhere')
                    owner.document.disableSetModified()
                    try:
                        owner.validate(review, self.deadline, own_notification_batch=True)
                        wrote = True
                        review['range'].String = replacement
                    finally:
                        owner.document.enableSetModified()
                        if wrote:
                            owner.document.setModified(True)
                finally:
                    undo.leaveUndoContext()
                if owner.read_range(review['range']) != replacement:
                    value = dict(status='UNKNOWN_CHANGED', reason='postwrite_mismatch')
                else:
                    value = dict(status='APPLIED', undo=bool(undo.isUndoPossible()))
            else:
                raise Refused('action')
        except Refused as error:
            value = dict(status='UNKNOWN_CHANGED' if wrote else 'REJECTED', reason=str(error))
        except Exception as error:
            value = dict(status='UNKNOWN_CHANGED' if wrote else 'ERROR', reason=type(error).__name__ + ': ' + str(error))
        finally:
            self.args = None
            if owner.revoked:
                try:
                    owner.disable()
                    if wrote:
                        value = dict(status='UNKNOWN_CHANGED', reason='revoked_after_write')
                except Exception as error:
                    value = dict(status='UNKNOWN_CHANGED' if wrote else 'ERROR', reason='detach: ' + str(error))
        owner.result = value
        owner.pending = False


class Writer(unohelper.Base, XJob):
    def __init__(self, context):
        if os.environ.get('USERNAME') != 'WDAGUtilityAccount' or os.environ.get('GRAMMAR_WRITER_FIXTURE') != '1':
            raise RuntimeError('isolated synthetic fixture only')
        self.context = context
        self.desktop = context.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', context)
        self.document = self.controller = self.listener = self.review = self.result = None
        self.revision = self.selection_revision = 0
        self.pending = False
        self.expiry_timer = None
        self.text_reads = 0
        self.epoch = 0
        self.revoked = True

    def deadline(self, expiry):
        if time.monotonic() >= expiry:
            raise Refused('expired_operation')

    def clear_review(self):
        self.review = None
        if self.expiry_timer:
            self.expiry_timer.cancel()
            self.expiry_timer = None

    def schedule_expiry(self, token):
        def post():
            callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
            callback.addCallback(Expiry(self, token), None)
        self.expiry_timer = threading.Timer(30, post)
        self.expiry_timer.daemon = True
        self.expiry_timer.start()

    def disable(self):
        self.revoked = True
        self.clear_review()
        if self.listener:
            try:
                self.document.removeModifyListener(self.listener)
                self.controller.removeSelectionChangeListener(self.listener)
            finally:
                self.listener.owner = None
                self.listener = None
        self.document = self.controller = None

    def check_document(self, doc, own_notification_batch=False):
        if doc is None:
            raise Refused('disabled')
        # Only new unsaved plain-body fixture documents. Not a policy for arbitrary user files.
        if not doc.supportsService('com.sun.star.text.TextDocument') or doc.hasLocation() or doc.isReadonly():
            raise Refused('unsupported_document')
        if not own_notification_batch and not doc.isSetModifiedEnabled():
            raise Refused('modification_state_owned_elsewhere')
        if doc.TextSections.Count or doc.TextTables.Count or doc.TextFrames.Count or doc.DrawPage.Count:
            raise Refused('unsupported_structure')
        if self.desktop.CurrentComponent != doc:
            raise Refused('document_changed')
        controller = doc.CurrentController
        if self.controller is not None and controller != self.controller:
            raise Refused('controller_changed')
        hwnd = int(controller.Frame.ContainerWindow.getWindowHandle((), SYSTEM_WIN32))
        pid = ctypes.c_ulong()
        api = ctypes.windll.user32.GetWindowThreadProcessId
        api.argtypes = (ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong))
        api.restype = ctypes.c_ulong
        tid = api(ctypes.c_void_p(hwnd), ctypes.byref(pid))
        if pid.value != os.getpid() or tid != threading.get_native_id():
            raise Refused('wrong_thread')

    def selected(self):
        selection = self.controller.getSelection()
        if selection.getCount() != 1:
            raise Refused('multiple_ranges')
        selected = selection.getByIndex(0)
        if selected.Text != self.document.Text:
            raise Refused('non_body_range')
        return selected

    def read_range(self, selected):
        self.text_reads += 1
        return selected.String

    def validate(self, review, deadline, own_notification_batch=False):
        self.permission(review['epoch'])
        self.deadline(deadline)
        if time.monotonic() >= review['expires']:
            raise Refused('expired_review')
        self.check_document(self.document, own_notification_batch)
        if self.revision != review['revision'] or self.selection_revision != review['selection_revision']:
            raise Refused('revision_changed')
        selected = self.selected()
        text = self.document.Text
        if text.compareRegionStarts(selected, review['range']) or text.compareRegionEnds(selected, review['range']):
            raise Refused('range_changed')
        if self.read_range(selected) != review['source'] or self.read_range(review['range']) != review['source']:
            raise Refused('source_changed')
        self.deadline(deadline)
        self.permission(review['epoch'])

    def permission(self, epoch):
        if self.revoked or self.epoch != epoch:
            raise Refused('permission_revoked')

    def execute(self, arguments):
        args = {arg.Name: arg.Value for arg in arguments}
        if args.get('Action') == 'state':
            return json.dumps(dict(status='STATE', reviewRetained=self.review is not None,
                resultRetained=self.result is not None, enabled=self.document is not None and not self.revoked,
                bindingRetained=self.document is not None, revoked=self.revoked,
                pending=self.pending, textReads=self.text_reads))
        if args.get('Action') == 'poll':
            value = self.result
            self.result = None
            return json.dumps(value or dict(status='PENDING'))
        if args.get('Action') == 'disable':
            # Revoke immediately even during an in-flight callback; detachment belongs
            # to its UI-thread finally block. REVOKING is not a cleanup-completed claim.
            self.epoch += 1
            self.revoked = True
            if self.pending:
                return json.dumps(dict(status='REVOKING'))
            self.result = None
        if self.pending or self.result is not None:
            return json.dumps(dict(status='BUSY'))
        expiry = args.get('Deadline')
        if not isinstance(expiry, (float, int)) or not math.isfinite(expiry) or not 0 < expiry - time.monotonic() <= 3:
            return json.dumps(dict(status='REJECTED', reason='deadline'))
        self.pending = True
        try:
            callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
            callback.addCallback(Callback(self, args, expiry, self.epoch), None)
        except Exception:
            self.pending = False
            raise
        return json.dumps(dict(status='QUEUED'))


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(Writer, 'org.grammar.fixture.Writer', ('org.grammar.fixture.Writer',))
