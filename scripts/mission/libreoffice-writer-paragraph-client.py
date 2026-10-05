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
result = dict(status='RUNNING', scope='experimental paragraph-aware Writer transaction; not product qualified', eventHarness='target_internal', checks=[])
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
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.ParagraphWriter', ctx)
    assert service is not None
    texts = ['First paragraph', 'Before | Synthetic 한국어 😀 | After', '', 'Last paragraph']
    colors = [0x118822, 0xBB2200, 0x2255AA, 0x442299]
    aligns = [0, 3, 0, 1]
    replacement = 'Reviewed 한국어 😀'

    def paragraphs():
        items = []
        enum = doc.Text.createEnumeration()
        while enum.hasMoreElements(): items.append(enum.nextElement())
        return items

    def snapshot():
        rows = []
        for para in paragraphs():
            values = []
            empty = []
            enum = para.createEnumeration()
            while enum.hasMoreElements():
                part = enum.nextElement()
                fmt = (part.CharWeight, part.CharWeightAsian, part.CharWeightComplex, part.CharColor)
                values.extend([fmt] * len(part.String))
                if not part.String and not para.String: empty.append(fmt)
            rows.append((para.String, para.ParaAdjust, values, empty))
        return rows

    def reset():
        call('disable')
        doc.Text.String = texts[0]
        for text in texts[1:]:
            doc.Text.insertControlCharacter(doc.Text.End, PARAGRAPH_BREAK, False)
            doc.Text.insertString(doc.Text.End, text, False)
        for index, para in enumerate(paragraphs()):
            cursor = doc.Text.createTextCursorByRange(para.Start)
            cursor.gotoRange(para.End, True)
            cursor.setAllPropertiesToDefault()
            cursor.ParaAdjust = aligns[index]
            cursor.CharColor = colors[index]
            cursor.CharWeight = cursor.CharWeightAsian = cursor.CharWeightComplex = 150.0
        doc.getUndoManager().clear()
        assert [p.String for p in paragraphs()] == texts

    def choose(index, start, count):
        para = paragraphs()[index]
        cursor = doc.Text.createTextCursorByRange(para.Start)
        cursor.goRight(start, False)
        cursor.goRight(count, True)
        doc.CurrentController.select(cursor)
        return cursor

    def ready(index=1, start=9, count=9):
        choose(index, start, count)
        result['bodyRepresentation'] = repr(doc.Text.String)
        result['paragraphLengths'] = [len(p.String) for p in paragraphs()]
        out.write_text(json.dumps(result, indent=2), encoding='utf-8')
        focus_owned_document()
        enabled = call('enable', Document=doc)
        assert enabled['status'] == 'ENABLED', enabled
        captured = call('capture')
        assert captured['status'] == 'CAPTURED', captured
        return captured['token']

    def exercise(index, start, count, tag):
        before = snapshot()
        selected = choose(index, start, count)
        original = selected.String
        prefix = doc.Text.createTextCursorByRange(paragraphs()[index].Start)
        prefix.gotoRange(selected.Start, True)
        offset = len(prefix.String)
        expected_text = before[index][0][:offset] + replacement + before[index][0][offset + len(original):]
        expected = list(before)
        expected_formats = before[index][2][:offset] + [before[index][2][offset]] * len(replacement) + before[index][2][offset + len(original):]
        expected[index] = (expected_text, before[index][1], expected_formats, [])
        token = ready(index, start, count)
        answer = call('accept', Token=token, Replacement=replacement)
        result['lastReply'] = answer
        record(tag+'-apply-all-paragraph-text-and-format', answer['status']=='APPLIED' and snapshot()==expected)
        doc.getUndoManager().undo()
        record(tag+'-undo-all-paragraphs', snapshot()==before)
        doc.getUndoManager().redo()
        record(tag+'-redo-all-paragraphs', snapshot()==expected)
        doc.getUndoManager().undo()
        assert snapshot()==before

    reset()
    for index,start,count,tag in [(0,0,5,'first'),(1,9,9,'middle'),(3,5,9,'last')]:
        exercise(index,start,count,tag)
    choose(1, 0, 6).CharColor = 0x118822
    exercise(1, 9, 9, 'mixed-within-middle')
    token = ready()
    other = doc.Text.createTextCursorByRange(paragraphs()[0])
    other.ParaAdjust = 1
    preserved = snapshot()
    answer = call('accept', Token=token, Replacement=replacement)
    record('outside-paragraph-format-change-refuses-stale', answer['status']=='REJECTED' and snapshot()==preserved)
    reset()
    token = ready()
    assert call('accept', Token=token, Replacement=replacement)['status'] == 'APPLIED'
    manager = doc.getUndoManager()
    manager.lock()
    try:
        empty = doc.Text.createTextCursorByRange(paragraphs()[2].Start)
        empty.CharColor = 0x118899
    finally:
        manager.unlock()
    preserved = snapshot()
    assert preserved[2][3][0][3] == 0x118899, 'empty paragraph change fixture'
    error_message = None
    try:
        manager.undo()
    except Exception as error:
        error_message = str(error)
    record('changed-empty-paragraph-typing-format-blocks-undo', error_message is not None
        and 'Grammar document changed; no replacement' in error_message and snapshot() == preserved)
    reset()
    preserved_body = doc.Text.String
    cursor=doc.Text.createTextCursorByRange(paragraphs()[0].Start)
    cursor.gotoRange(paragraphs()[1].End,True)
    doc.CurrentController.select(cursor)
    focus_owned_document()
    assert call('enable',Document=doc)['status']=='ENABLED'
    answer=call('capture')
    record('cross-paragraph-selection-refused',answer['status']=='REJECTED' and doc.Text.String==preserved_body)
    reset()
    before=snapshot()
    file=Path(r'C:\GrammarOutput\paragraphs-synthetic.docx')
    url=uno.systemPathToFileUrl(str(file))
    doc.storeAsURL(url,(arg('PropertyValue','FilterName','Office Open XML Text'),))
    keeper=desktop.loadComponentFromURL('private:factory/swriter','_blank',0,())
    doc.close(True)
    doc=desktop.loadComponentFromURL(url,'_blank',0,(arg('PropertyValue','MacroExecutionMode',NEVER_EXECUTE),arg('PropertyValue','UpdateDocMode',0)))
    keeper.close(True)
    result['beforeSaveSnapshot'] = before
    result['afterReopenSnapshot'] = snapshot()
    observe_failure('saved-reopened-all-paragraphs-exact',snapshot()==before)
    digest=hashlib.sha256(file.read_bytes()).hexdigest()
    for cycle in range(8): exercise(1,9,9,'saved-'+str(cycle))
    record('no-automatic-file-save',hashlib.sha256(file.read_bytes()).hexdigest()==digest)
    result['status']='FAIL' if result.get('failures') else 'PASS'

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
