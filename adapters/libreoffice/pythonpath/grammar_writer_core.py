"""Writer transaction core. Not a registered UNO service or an enabled adapter.

A deployment entry point must supply authenticated transport and explicit document
opt-in. Current qualification is owned unsaved, bounded, plain-body fixtures only.
"""
import ctypes
import json
import hashlib
import math
import os
import secrets
import threading
import time
import uno
import unohelper
from com.sun.star.lang import XComponent
from com.sun.star.task import XJob
from com.sun.star.document import XUndoAction, UndoFailedException
from com.sun.star.awt import XCallback
from com.sun.star.util import XModifyListener
from com.sun.star.view import XSelectionChangeListener
from com.sun.star.lang.SystemDependent import SYSTEM_WIN32
from com.sun.star.beans.PropertyAttribute import READONLY


class Refused(Exception):
    pass


def plain_metadata(doc):
    if (doc.TextSections.Count or doc.TextTables.Count or doc.TextFrames.Count or doc.DrawPage.Count
            or doc.TextFields.createEnumeration().hasMoreElements() or doc.Bookmarks.hasElements()
            or doc.Redlines.Count or doc.RecordChanges):
        raise Refused('unsupported_structure')
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(16385, True)
    if doc.Text.compareRegionEnds(cursor, doc.Text.End) != 0:
        raise Refused('body_bounds')
    if cursor.getPropertyState('CharHidden').value == 'AMBIGUOUS_VALUE' or cursor.CharHidden:
        raise Refused('hidden_text')
    # The custom Undo action restores text, not arbitrary rich formatting.
    # Inspect actual portions, not aggregate cursor property states: Writer can
    # report DEFAULT_VALUE there even after direct formatting was assigned.
    style = doc.StyleFamilies.getByName('ParagraphStyles').getByName('Standard')
    characters = ('CharFontName', 'CharFontNameAsian', 'CharFontNameComplex',
        'CharHeight', 'CharHeightAsian', 'CharHeightComplex',
        'CharWeight', 'CharWeightAsian', 'CharWeightComplex',
        'CharPosture', 'CharPostureAsian', 'CharPostureComplex',
        'CharColor', 'CharBackColor', 'CharUnderline', 'CharStrikeout',
        'CharEscapement', 'CharKerning', 'CharAutoKerning', 'CharScaleWidth',
        'CharCaseMap', 'CharRelief', 'CharShadowed', 'CharContoured', 'CharRotation', 'CharEmphasis')
    paragraphs = ('ParaAdjust', 'ParaLeftMargin', 'ParaRightMargin', 'ParaTopMargin',
        'ParaBottomMargin', 'ParaFirstLineIndent')
    defaults = {name: style.getPropertyValue(name) for name in characters + paragraphs}
    enumeration = doc.Text.createEnumeration()
    paragraph_count = portion_count = 0
    while enumeration.hasMoreElements():
        paragraph_count += 1
        if paragraph_count > 256:
            raise Refused('paragraph_bounds')
        paragraph = enumeration.nextElement()
        if paragraph.ParaStyleName != 'Standard' or paragraph.NumberingStyleName:
            raise Refused('unsupported_paragraph_style')
        for name in paragraphs:
            if paragraph.getPropertyValue(name) != defaults[name]:
                raise Refused('unsupported_format_' + name)
        portions = paragraph.createEnumeration()
        while portions.hasMoreElements():
            portion_count += 1
            if portion_count > 1024:
                raise Refused('portion_bounds')
            portion = portions.nextElement()
            if portion.TextPortionType != 'Text' or portion.CharStyleName or portion.HyperLinkURL or portion.CharHidden:
                raise Refused('unsupported_text_portion')
            for property_info in portion.getPropertySetInfo().getProperties():
                name = property_info.Name
                if name.startswith('Char') and not (property_info.Attributes & READONLY):
                    if portion.getPropertyState(name).value != 'DEFAULT_VALUE':
                        raise Refused('unsupported_character_format')
            for name in characters:
                if portion.getPropertyValue(name) != defaults[name]:
                    raise Refused('unsupported_format_' + name)


def body_snapshot(doc):
    plain_metadata(doc)
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(16385, True)
    if doc.Text.compareRegionEnds(cursor, doc.Text.End) != 0:
        raise Refused('body_bounds')
    value = cursor.String
    if len(value) > 16384:
        raise Refused('body_bounds')
    return value


def fingerprint(value):
    return hashlib.sha256(value.encode('utf-8')).digest()


def movement_to(doc, target):
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    for steps in range(16385):
        if doc.Text.compareRegionStarts(cursor, target) == 0:
            return steps
        if not cursor.goRight(1, False):
            break
    raise Refused('unrepresentable_range')


class ReplacementUndo(unohelper.Base, XUndoAction, XComponent):
    active = 0
    """Plain-body experiment: one range replacement, never whole-document restore."""
    def __init__(self, document, selected, before, after, start, before_steps, before_hash, after_hash):
        self.document = document
        self.before, self.after = before, after
        self.start = start
        self.before_steps = before_steps
        self.after_steps = movement_to(document, selected.End) - start
        self.before_hash, self.after_hash = before_hash, after_hash
        self.Title = 'Grammar reviewed replacement'
        self.disposed = False
        self.listeners = []
        ReplacementUndo.active += 1

    def dispose(self):
        if self.disposed:
            return
        self.disposed = True
        self.document = None
        self.before = self.after = None
        self.before_hash = self.after_hash = None
        ReplacementUndo.active -= 1
        listeners, self.listeners = self.listeners, []
        event = uno.createUnoStruct('com.sun.star.lang.EventObject')
        event.Source = self
        for listener in listeners:
            try:
                listener.disposing(event)
            except Exception:
                pass

    def addEventListener(self, listener):
        if self.disposed:
            event = uno.createUnoStruct('com.sun.star.lang.EventObject')
            event.Source = self
            listener.disposing(event)
        else:
            self.listeners.append(listener)

    def removeEventListener(self, listener):
        if listener in self.listeners:
            self.listeners.remove(listener)

    def replace(self, expected, replacement, steps, digest):
        if self.disposed:
            raise UndoFailedException('Disposed Grammar action', self, None)
        doc = self.document
        if fingerprint(body_snapshot(doc)) != digest or not doc.isSetModifiedEnabled():
            raise UndoFailedException('Grammar document changed; no replacement', self, None)
        selected = doc.Text.createTextCursor()
        selected.gotoStart(False)
        if not selected.goRight(self.start, False) or not selected.goRight(steps, True) or selected.String != expected:
            raise UndoFailedException('Grammar range changed; no replacement', self, None)
        manager = doc.getUndoManager()
        # Native Undo/Redo owns the recording lock while invoking XUndoAction.
        if not manager.isLocked():
            raise UndoFailedException('Undo manager must own recording lock', self, None)
        doc.disableSetModified()
        try:
            if selected.String != expected:
                raise UndoFailedException('Grammar range changed', self, None)
            selected.String = replacement
        finally:
            doc.enableSetModified()
            doc.setModified(True)
        if selected.String != replacement:
            raise UndoFailedException('Postwrite result changed', self, None)

    def undo(self):
        self.replace(self.after, self.before, self.after_steps, self.after_hash)

    def redo(self):
        self.replace(self.before, self.after, self.before_steps, self.before_hash)


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
                owner.window = int(owner.controller.Frame.ContainerWindow.getWindowHandle((), SYSTEM_WIN32))
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
                owner.foreground_document()
                selected = owner.selected()
                source = owner.read_range(selected)
                if not source or len(source) > 4096:
                    raise Refused('source_bounds')
                owner.deadline(self.deadline)
                owner.permission(self.epoch)
                owner.foreground_document()
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
                    # Record one custom action after replacement, while native
                    # sub-operation notifications/recording are suppressed. Existing
                    # history is retained; no clear/reset or whole-document rollback.
                    if undo.isLocked():
                        raise Refused('undo_owned_elsewhere')
                    # The bounded local body hash qualifies custom Undo/Redo
                    # after earlier native history rebuilds live range objects.
                    before_body = body_snapshot(owner.document)
                    start = movement_to(owner.document, review['range'].Start)
                    before_steps = movement_to(owner.document, review['range'].End) - start
                    before_hash = fingerprint(before_body)
                    before_body = None
                    undo.lock()
                    try:
                        owner.validate(review, self.deadline)
                        owner.document.disableSetModified()
                        try:
                            owner.validate(review, self.deadline, own_notification_batch=True)
                            wrote = True
                            review['range'].String = replacement
                        finally:
                            owner.document.enableSetModified()
                    finally:
                        undo.unlock()
                    undo.addUndoAction(ReplacementUndo(owner.document, review['range'], review['source'], replacement,
                        start, before_steps, before_hash, fingerprint(body_snapshot(owner.document))))
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


class WriterCore(unohelper.Base, XJob):
    def __init__(self, context):
        self.context = context
        self.desktop = context.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', context)
        self.document = self.controller = self.listener = self.review = self.result = None
        self.revision = self.selection_revision = 0
        self.pending = False
        self.expiry_timer = None
        self.text_reads = 0
        self.epoch = 0
        self.revoked = True
        self.window = 0

    def deadline(self, expiry):
        ticks = None
        if isinstance(expiry, tuple):
            expiry, ticks = expiry
        if ticks is not None:
            tick_clock = ctypes.windll.kernel32.GetTickCount64
            tick_clock.restype = ctypes.c_ulonglong
            if tick_clock() >= ticks:
                raise Refused('expired_operation')
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
        plain_metadata(doc)
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

    def foreground_document(self):
        foreground = ctypes.windll.user32.GetForegroundWindow
        foreground.restype = ctypes.c_void_p
        if foreground() != self.window:
            raise Refused('document_not_foreground')

    def read_range(self, selected):
        bounded = self.document.Text.createTextCursorByRange(selected.Start)
        bounded.goRight(4096, True)
        if self.document.Text.compareRegionEnds(bounded, selected) > 0:
            raise Refused('source_bounds')
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
                pending=self.pending, textReads=self.text_reads, undoActions=ReplacementUndo.active))
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
        ticks = args.get('DeadlineTicks')
        if ticks is not None:
            tick_clock = ctypes.windll.kernel32.GetTickCount64
            tick_clock.restype = ctypes.c_ulonglong
            if isinstance(ticks, bool) or not isinstance(ticks, int) or not 0 < ticks - tick_clock() <= 3000:
                return json.dumps(dict(status='REJECTED', reason='deadline_ticks'))
            expiry = (expiry, ticks)
        self.pending = True
        try:
            callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
            callback.addCallback(Callback(self, args, expiry, self.epoch), None)
        except Exception:
            self.pending = False
            raise
        return json.dumps(dict(status='QUEUED'))

