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
result = dict(status='RUNNING', scope='experimental uniform-format Writer transaction; not product qualified', eventHarness='target_internal', checks=[])
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
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.FormattedWriter', ctx)
    assert service is not None
    replacement = 'Reviewed 한국어 😀'

    def whole():
        cursor = doc.Text.createTextCursor()
        cursor.gotoEnd(True)
        return cursor

    def formatting():
        # Read actual portions, not aggregate cursor states. Every character must
        # retain the uniform format, including unselected prefix and suffix.
        rows = []
        paragraphs = doc.Text.createEnumeration()
        while paragraphs.hasMoreElements():
            para = paragraphs.nextElement()
            portions = para.createEnumeration()
            while portions.hasMoreElements():
                part = portions.nextElement()
                rows.append((part.CharWeight, part.CharColor, para.ParaAdjust, part.CharWeightAsian, part.CharWeightComplex))
        return rows

    def reset():
        call('disable')
        doc.Text.String = source
        cursor = whole()
        cursor.setAllPropertiesToDefault()
        cursor.CharWeight = cursor.CharWeightAsian = cursor.CharWeightComplex = 150.0
        cursor.CharColor = 0x2255AA
        cursor.ParaAdjust = 3

    def formatted_capture(start=9, count=9):
        select(start, count)
        focus_owned_document()
        enabled = call('enable', Document=doc)
        assert enabled['status'] == 'ENABLED', enabled
        captured = call('capture')
        assert captured['status'] == 'CAPTURED', captured
        return captured['token']

    reset()
    record('actual-bold-color-centered-fixture', all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    for label, start, count in [('middle', 9, 9), ('prefix', 0, 6), ('suffix', 27, 5), ('whole', 0, 32)]:
        reset()
        selected = select(start, count)
        original = selected.String
        assert original and source.count(original) == 1, (label, original)
        expected = source.replace(original, replacement)
        paragraph = doc.Text.createEnumeration().nextElement()
        portion = paragraph.createEnumeration().nextElement()
        result['beforeStates'] = {name: portion.getPropertyState(name).value
            for name in ('CharWeight', 'CharWeightAsian', 'CharWeightComplex', 'CharColor')}
        result['beforeParaState'] = paragraph.getPropertyState('ParaAdjust').value
        token = formatted_capture(start, count)
        response = call('accept', Token=token, Replacement=replacement)
        result['lastReply'] = response
        diagnostic = []
        parts = doc.Text.createEnumeration().nextElement().createEnumeration()
        while parts.hasMoreElements():
            part = parts.nextElement()
            diagnostic.append({prop.Name: str(part.getPropertyValue(prop.Name))
                for prop in part.getPropertySetInfo().getProperties()
                if prop.Name.startswith('Char') and not prop.Attributes & READONLY
                and part.getPropertyState(prop.Name).value != 'DEFAULT_VALUE'})
        result['postwriteFormatDiagnostic'] = diagnostic
        result['postwriteUniformValues'] = formatting()
        result['afterParaState'] = doc.Text.createEnumeration().nextElement().getPropertyState('ParaAdjust').value
        record(label + '-apply-exact-and-all-portions-formatted', response['status'] == 'APPLIED'
            and doc.Text.String == expected and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
        doc.getUndoManager().undo()
        record(label + '-undo-text-and-format', doc.Text.String == source
            and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
        doc.getUndoManager().redo()
        record(label + '-redo-text-and-format', doc.Text.String == expected
            and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    reset()
    token = formatted_capture()
    whole().CharColor = 0x880022
    record('user-format-change-rejects-stale-accept', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source and all(row[1] == 0x880022 for row in formatting()))
    reset()
    token = formatted_capture()
    style = doc.StyleFamilies.getByName('ParagraphStyles').getByName('Standard')
    height = style.CharHeight
    try:
        style.CharHeight = height + 1
        record('inherited-style-change-rejects-stale-accept',
            call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
            and doc.Text.String == source and style.CharHeight == height + 1)
    finally:
        style.CharHeight = height
    reset()
    token = formatted_capture()
    values = dict(Action='arm', Mode='format', Token=token, Document=doc, Service=service, UserText='UNUSED')
    hooks.execute(tuple(arg('NamedValue', k, v) for k, v in values.items()))
    try:
        response = call('accept', Token=token, Replacement='WRONG')
    finally:
        hook = json.loads(hooks.execute((arg('NamedValue', 'Action', 'disarm'),)))
    record('format-change-inside-undo-entry-preserved', hook['fired'] and not hook.get('hookError')
        and response['status'] == 'REJECTED' and doc.Text.String == source
        and all(row[1] == 0x880022 for row in formatting()))
    reset()
    token = formatted_capture()
    record('newline-refused-without-text-format-change', call('accept', Token=token, Replacement='bad\nparagraph')['status'] == 'REJECTED'
        and doc.Text.String == source and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    reset()
    select().CharColor = 0x880022
    reads = call('state')['textReads']
    record('mixed-character-format-refused-before-capture', call('enable', Document=doc)['status'] == 'REJECTED'
        and call('state')['textReads'] == reads and doc.Text.String == source)
    reset()
    whole().CharColorTheme = 4
    reads = call('state')['textReads']
    record('theme-color-refused-before-capture', call('enable', Document=doc)['status'] == 'REJECTED'
        and call('state')['textReads'] == reads)
    reset()
    item = arg('PropertyValue', 'UnknownInterop', 'synthetic')
    uno.invoke(whole(), 'setPropertyValue', ('CharInteropGrabBag',
        uno.Any('[]com.sun.star.beans.PropertyValue', (item,))))
    reads = call('state')['textReads']
    record('unknown-interop-refused-before-capture', call('enable', Document=doc)['status'] == 'REJECTED'
        and call('state')['textReads'] == reads)
    reset()
    doc.Text.insertControlCharacter(doc.Text.End, PARAGRAPH_BREAK, False)
    doc.Text.insertString(doc.Text.End, 'Second synthetic paragraph', False)
    paragraphs = doc.Text.createEnumeration()
    paragraph_count = 0
    while paragraphs.hasMoreElements():
        paragraphs.nextElement()
        paragraph_count += 1
    record('actual-two-paragraph-fixture', paragraph_count == 2)
    reads = call('state')['textReads']
    result['multipleParagraphReply'] = call('enable', Document=doc)
    observe_failure('multiple-paragraphs-refused-before-capture', result['multipleParagraphReply']['status'] == 'REJECTED'
        and call('state')['textReads'] == reads)
    reset()
    whole().CharHidden = True
    reads = call('state')['textReads']
    record('hidden-formatted-field-refused-before-capture', call('enable', Document=doc)['status'] == 'REJECTED'
        and call('state')['textReads'] == reads)
    whole().CharHidden = False
    reset()
    token = formatted_capture()
    response = call('accept', Token=token, Replacement=replacement)
    assert response['status'] == 'APPLIED', response
    # App formatting made with undo recording locked must not be overwritten by
    # Grammar's old custom Undo. This tests the fingerprint, not native history order.
    manager = doc.getUndoManager()
    manager.lock()
    try:
        whole().CharColor = 0x880022
    finally:
        manager.unlock()
    try:
        manager.undo()
        undo_refused = False
    except Exception as error:
        result['staleUndoError'] = str(error)
        undo_refused = 'Grammar document changed; no replacement' in str(error)
    record('out-of-band-format-change-blocks-custom-undo', undo_refused
        and doc.Text.String == source.replace('Synthetic', replacement)
        and all(row[1] == 0x880022 for row in formatting()))
    reset()
    for index in range(16):
        token = formatted_capture()
        reply = call('accept', Token=token, Replacement=replacement)
        assert reply['status'] == 'APPLIED' and doc.Text.String == source.replace('Synthetic', replacement), (index, reply)
        doc.getUndoManager().undo()
        assert doc.Text.String == source and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()), index
        result['formattedCycles'] = index + 1
    record('sixteen-formatted-apply-undo-cycles', True)
    file = Path(r'C:\GrammarOutput\writer-owned-formatted.docx')
    url = uno.systemPathToFileUrl(str(file))
    doc.storeAsURL(url, (arg('PropertyValue', 'FilterName', 'Office Open XML Text'), arg('PropertyValue', 'Overwrite', False)))
    # Keep a second owned document alive while reopening.
    keeper = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0, ())
    doc.close(True)
    doc = desktop.loadComponentFromURL(url, '_blank', 0,
        (arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE), arg('PropertyValue', 'UpdateDocMode', 0)))
    keeper.close(True)
    result['savedReopenedFormat'] = formatting()
    parts = doc.Text.createEnumeration().nextElement().createEnumeration()
    diagnostic = []
    while parts.hasMoreElements():
        part = parts.nextElement()
        diagnostic.append({prop.Name: str(part.getPropertyValue(prop.Name))
            for prop in part.getPropertySetInfo().getProperties()
            if prop.Name.startswith('Char') and not prop.Attributes & READONLY
            and part.getPropertyState(prop.Name).value != 'DEFAULT_VALUE'})
    result['savedFormatStates'] = diagnostic
    result['savedReopenedTextMatches'] = doc.Text.String == source
    record('saved-reopened-format-exact', doc.Text.String == source
        and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    digest = hashlib.sha256(file.read_bytes()).hexdigest()
    token = formatted_capture()
    response = call('accept', Token=token, Replacement=replacement)
    result['savedReply'] = response
    record('saved-formatted-docx-apply-exact', response['status'] == 'APPLIED'
        and doc.Text.String == source.replace('Synthetic', replacement)
        and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    doc.getUndoManager().undo()
    record('saved-formatted-docx-undo-exact', doc.Text.String == source
        and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    record('saved-formatted-docx-not-automatically-saved', hashlib.sha256(file.read_bytes()).hexdigest() == digest)
    doc.getUndoManager().redo()
    record('saved-formatted-docx-redo-exact', doc.Text.String == source.replace('Synthetic', replacement)
        and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    # Explicit synthetic user Save, never a product-triggered store.
    doc.store()
    record('explicit-save-updates-synthetic-file', hashlib.sha256(file.read_bytes()).hexdigest() != digest)
    keeper = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0, ())
    doc.close(True)
    doc = desktop.loadComponentFromURL(url, '_blank', 0,
        (arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE), arg('PropertyValue', 'UpdateDocMode', 0)))
    keeper.close(True)
    record('explicit-save-reopen-preserves-replacement-and-format', doc.Text.String == source.replace('Synthetic', replacement)
        and all(row == (150.0, 0x2255AA, 3, 150.0, 150.0) for row in formatting()))
    result['status'] = 'FAIL' if result.get('failures') else 'PASS'

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
