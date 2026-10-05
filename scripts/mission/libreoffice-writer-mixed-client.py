"""Real Writer transaction fixture client; invoke only through guarded Sandbox launcher."""
import json
import hashlib
import ctypes
import subprocess
import os
from pathlib import Path
import sys
import time
from types import SimpleNamespace
if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
    raise RuntimeError('Guest only')
import uno
from com.sun.star.document.MacroExecMode import NEVER_EXECUTE
from com.sun.star.lang import DisposedException
from com.sun.star.beans.PropertyAttribute import READONLY
from com.sun.star.text.ControlCharacter import PARAGRAPH_BREAK

out = Path(r'C:\GrammarOutput\uno-result.json')
result = dict(status='RUNNING', scope='experimental mixed-surroundings Writer transaction; not product qualified', eventHarness='target_internal', checks=[])
desktop = doc = service = None
source = 'Before | Synthetic 한국어 😀 | After'


def arg(kind, name, value):
    item = uno.createUnoStruct('com.sun.star.beans.' + kind)
    item.Name, item.Value = name, value
    return item


def record(name, condition):
    assert condition, name
    result['checks'].append(name)
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')


def observe_failure(name, condition):
    # Keep independent counterexamples running; a failed assertion still makes the suite FAIL.
    if condition:
        record(name, True)
    else:
        result.setdefault('failures', []).append(name)
        out.write_text(json.dumps(result, indent=2), encoding='utf-8')


def call(action, **values):
    values.update(Action=action)
    values.setdefault('Deadline', time.monotonic() + 2.5)
    answer = json.loads(service.execute(tuple(arg('NamedValue', k, v) for k, v in values.items())))
    if answer['status'] != 'QUEUED':
        return answer
    limit = time.monotonic() + 5
    while time.monotonic() < limit:
        answer = json.loads(service.execute((arg('NamedValue', 'Action', 'poll'),)))
        if answer['status'] != 'PENDING':
            return answer
        time.sleep(.01)
    raise AssertionError('queue completion deadline')


def select(start=9, count=9):
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(start, False)
    cursor.goRight(count, True)
    doc.CurrentController.select(cursor)
    return cursor


def capture():
    doc.Text.String = source
    doc.setModified(False)
    select()
    focus_owned_document()
    enabled = call('enable', Document=doc)
    assert enabled['status'] == 'ENABLED', enabled
    captured = call('capture')
    assert captured['status'] == 'CAPTURED' and captured['source'] == 'Synthetic', captured
    return captured['token']


def focus_owned_document():
    # Setup only, before the operation's deadline. Never infer foreground from
    # successful window activation; assert the actual exact synthetic HWND.
    hwnd = int(doc.CurrentController.Frame.ContainerWindow.getWindowHandle((), 1))
    api = ctypes.windll.user32
    api.SetForegroundWindow.argtypes = [ctypes.c_void_p]
    api.GetForegroundWindow.restype = ctypes.c_void_p
    end = time.monotonic() + 2
    while time.monotonic() < end:
        doc.CurrentController.Frame.activate()
        api.SetForegroundWindow(ctypes.c_void_p(hwnd))
        if api.GetForegroundWindow() == hwnd:
            return
        time.sleep(.02)
    raise AssertionError('owned Writer foreground setup unavailable')




try:
    out.write_text(json.dumps(result), encoding='utf-8')
    ctx = uno.getComponentContext()
    resolver = ctx.ServiceManager.createInstanceWithContext('com.sun.star.bridge.UnoUrlResolver', ctx)
    end = time.monotonic() + 30
    while True:
        try:
            ctx = resolver.resolve('uno:pipe,name=' + sys.argv[1] + ';urp;StarOffice.ComponentContext')
            break
        except Exception:
            if time.monotonic() >= end:
                raise
            time.sleep(.1)
    desktop = ctx.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', ctx)
    doc = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0,
        (arg('PropertyValue', 'Hidden', False), arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE)))
    assert doc.CurrentController.Frame.ContainerWindow.isVisible()
    doc.CurrentController.Frame.activate()
    ready_until = time.monotonic() + 5
    while desktop.CurrentComponent != doc and time.monotonic() < ready_until:
        time.sleep(.01)
    assert desktop.CurrentComponent == doc, 'owned Writer document not active after load'
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Writer', ctx)
    assert service is not None
    hooks = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Hooks', ctx)
    assert hooks is not None
    call('disable')
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.MixedWriter', ctx)
    assert service is not None
    source = 'Prefix | Synthetic 한국어 😀 | Suffix'
    replacement = 'Reviewed 한국어 😀'

    def whole():
        cursor = doc.Text.createTextCursor()
        cursor.gotoEnd(True)
        return cursor

    def snapshot():
        rows = []
        enumeration = doc.Text.createEnumeration()
        while enumeration.hasMoreElements():
            paragraph = enumeration.nextElement()
            portions = paragraph.createEnumeration()
            while portions.hasMoreElements():
                part = portions.nextElement()
                fmt = (part.CharWeight, part.CharWeightAsian, part.CharWeightComplex, part.CharColor, paragraph.ParaAdjust)
                rows.extend([fmt] * len(part.String))
        return rows

    def reset():
        call('disable')
        doc.Text.String = source
        whole().setAllPropertiesToDefault()
        whole().CharColor = 0x2255AA
        whole().ParaAdjust = 3
        mid = select(9, 9)
        mid.CharWeight = mid.CharWeightAsian = mid.CharWeightComplex = 150.0
        mid.CharColor = 0xBB2200
        doc.getUndoManager().clear()

    def ready(start=9, count=9):
        select(start, count)
        focus_owned_document()
        enabled = call('enable', Document=doc)
        assert enabled['status'] == 'ENABLED', enabled
        return call('capture')

    for label, start, count in [('full-run', 9, 9), ('inside-run', 10, 5), ('prefix', 0, 6), ('suffix', 27, 5)]:
        reset()
        cursor = select(start, count)
        original = cursor.String
        prefix = doc.Text.createTextCursor()
        prefix.gotoRange(cursor.Start, True)
        offset = len(prefix.String)
        before = snapshot()
        expected = source[:offset] + replacement + source[offset + len(original):]
        expected_format = before[:offset] + [before[offset]] * len(replacement) + before[offset + len(original):]
        captured = ready(start, count)
        assert captured['status'] == 'CAPTURED', captured
        reply = call('accept', Token=captured['token'], Replacement=replacement)
        result['lastReply'] = reply
        result['actualFormat'] = snapshot()
        record(label + '-apply-exact-text-and-surroundings', reply['status'] == 'APPLIED'
            and doc.Text.String == expected and snapshot() == expected_format)
        doc.getUndoManager().undo()
        record(label + '-undo-exact-text-and-surroundings', doc.Text.String == source and snapshot() == before)
        doc.getUndoManager().redo()
        record(label + '-redo-exact-text-and-surroundings', doc.Text.String == expected and snapshot() == expected_format)

    for start, count in [(8, 2), (17, 2), (0, 20)]:
        reset()
        before = snapshot()
        reply = ready(start, count)
        record('cross-format-refused-' + str(start), reply['status'] == 'REJECTED'
            and reply.get('reason') == 'selection_crosses_format_boundary'
            and doc.Text.String == source and snapshot() == before)

    reset()
    captured = ready()
    prefix = select(0, 6)
    prefix.CharColor = 0x118822
    preserved = snapshot()
    reply = call('accept', Token=captured['token'], Replacement=replacement)
    record('outside-format-change-refuses-stale-accept', reply['status'] == 'REJECTED'
        and doc.Text.String == source and snapshot() == preserved)

    reset()
    captured = ready()
    assert call('accept', Token=captured['token'], Replacement=replacement)['status'] == 'APPLIED'
    manager = doc.getUndoManager()
    manager.lock()
    try:
        select(0, 6).CharColor = 0x118822
    finally:
        manager.unlock()
    before_text, before_format = doc.Text.String, snapshot()
    failure = None
    try:
        manager.undo()
    except Exception as error:
        failure = str(error)
    record('outside-format-change-blocks-custom-undo', failure is not None
        and 'Grammar document changed; no replacement' in failure
        and doc.Text.String == before_text and snapshot() == before_format)

    reset()
    doc.Text.insertControlCharacter(doc.Text.End, PARAGRAPH_BREAK, False)
    doc.Text.insertString(doc.Text.End, 'Second paragraph', False)
    preserved = doc.Text.String
    focus_owned_document()
    reply = call('enable', Document=doc)
    record('multiple-paragraphs-still-refused', reply['status'] == 'REJECTED'
        and reply.get('reason') == 'formatted_single_paragraph_only' and doc.Text.String == preserved)

    reset()
    doc.Text.String = 'Before\nAfter'
    selected = whole()
    doc.CurrentController.select(selected)
    focus_owned_document()
    reply = call('enable', Document=doc)
    if reply['status'] == 'ENABLED':
        reply = call('capture')
    record('source-line-break-refused-before-write', reply['status'] == 'REJECTED'
        and reply.get('reason') == 'formatted_single_paragraph_only' and doc.Text.String == 'Before\nAfter')
    reset()
    original_format = snapshot()
    file = Path(r'C:\GrammarOutput\mixed-synthetic.docx')
    url = uno.systemPathToFileUrl(str(file))
    doc.storeAsURL(url, (arg('PropertyValue', 'FilterName', 'Office Open XML Text'),))
    keeper = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0, ())
    doc.close(True)
    doc = desktop.loadComponentFromURL(url, '_blank', 0,
        (arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE), arg('PropertyValue', 'UpdateDocMode', 0)))
    keeper.close(True)
    record('saved-reopened-mixed-fixture-exact', doc.Text.String == source and snapshot() == original_format)
    digest = hashlib.sha256(file.read_bytes()).hexdigest()
    for cycle in range(16):
        before = snapshot()
        captured = ready()
        assert captured['status'] == 'CAPTURED', captured
        reply = call('accept', Token=captured['token'], Replacement=replacement)
        expected_format = before[:9] + [before[9]] * len(replacement) + before[18:]
        record('saved-cycle-' + str(cycle), reply['status'] == 'APPLIED'
            and doc.Text.String == source.replace('Synthetic', replacement) and snapshot() == expected_format)
        doc.getUndoManager().undo()
        assert doc.Text.String == source and snapshot() == before
    record('saved-apply-undo-no-automatic-save', hashlib.sha256(file.read_bytes()).hexdigest() == digest)
    doc.getUndoManager().redo()
    record('saved-redo-exact', doc.Text.String == source.replace('Synthetic', replacement) and snapshot() == expected_format)
    call('disable')
    doc.store()
    keeper = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0, ())
    doc.close(True)
    doc = desktop.loadComponentFromURL(url, '_blank', 0,
        (arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE), arg('PropertyValue', 'UpdateDocMode', 0)))
    keeper.close(True)
    record('explicit-save-reopen-mixed-fidelity', doc.Text.String == source.replace('Synthetic', replacement)
        and snapshot() == expected_format)
    result['status'] = 'PASS'

except Exception as error:
    result.update(status='FAIL', error=type(error).__name__ + ': ' + str(error))
finally:
    try:
        if service:
            result['disable'] = call('disable')['status']
        if doc:
            doc.setModified(False)
            # Keep the final owned document alive until explicit termination.
            # Closing the last reopened document can naturally exit soffice and
            # disconnect URP before the following terminate call is delivered.
        try:
            result['desktopTerminated'] = bool(desktop.terminate()) if desktop else False
        except DisposedException:
            # The process may close URP before returning terminate's bool. This
            # is not cleanup completion: the launcher still requires actual
            # natural Office exit0, zero processes and removed profile/app.
            result['desktopTerminated'] = None
            result['terminationTransportClosed'] = True
            result['cleanupRequiresLauncherReceipt'] = True
    except Exception as error:
        result.update(status='FAIL', cleanupError=str(error))
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')
sys.exit(0 if result['status'] == 'PASS' else 1)
