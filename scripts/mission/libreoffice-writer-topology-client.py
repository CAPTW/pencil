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
from com.sun.star.text.ControlCharacter import PARAGRAPH_BREAK, LINE_BREAK

out = Path(r'C:\GrammarOutput\uno-result.json')
result = dict(status='RUNNING', scope='experimental uniform paragraph topology; not product qualified', eventHarness='target_internal', checks=[])
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
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.TopologyWriter', ctx)
    assert service is not None
    texts = ['Outside prefix', 'Alpha A', '', 'Beta B', 'Outside suffix']
    colors = [0x118822, 0xBB2200, 0xBB2200, 0xBB2200, 0x442299]
    aligns = [0, 3, 3, 3, 1]
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


    def ready_topology(first=1, last=3, a=3, b=4, required=True):
        cursor=doc.Text.createTextCursorByRange(paragraphs()[first].Start)
        cursor.goRight(a,False)
        end=doc.Text.createTextCursorByRange(paragraphs()[last].Start)
        end.goRight(b,False)
        cursor.gotoRange(end,True)
        doc.CurrentController.select(cursor)
        focus_owned_document()
        assert call('enable',Document=doc)['status']=='ENABLED'
        focus_owned_document()
        answer=call('capture')
        if not required:return answer
        assert answer['status']=='CAPTURED',answer
        return answer['token']

    def exercise(value,tag,first=1,last=3,a=3,b=4):
        before=snapshot()
        token=ready_topology(first,last,a,b)
        expected_texts=texts[:first]+(texts[first][:a]+value+texts[last][b:]).split('\n')+texts[last+1:]
        expected=[]
        for i,text in enumerate(expected_texts):
            # Independent per-character oracle; outside rows preserve distinct styles.
            fmt=before[0][2][0] if i==0 else before[-1][2][0] if i==len(expected_texts)-1 else before[1][2][0]
            alignment=before[0][1] if i==0 else before[-1][1] if i==len(expected_texts)-1 else before[1][1]
            expected.append((text,alignment,[fmt]*len(text),[fmt] if not text else []))
        answer=call('accept',Token=token,Replacement=value)
        result['lastReply']=answer
        result['actual']=snapshot();result['expected']=expected
        record(tag+'-apply',answer['status']=='APPLIED' and snapshot()==expected)
        doc.getUndoManager().undo()
        record(tag+'-undo',snapshot()==before)
        doc.getUndoManager().redo()
        record(tag+'-redo',snapshot()==expected)
        doc.getUndoManager().undo()
        assert snapshot()==before

    reset()
    for value,tag in [('Joined','merge'),('First\n\nSecond\nThird','split'),('\n','empty-parts')]:
        exercise(value,tag)
    exercise('one\ntwo','single-split',1,1,1,5)
    for mode in ('edit','delay','reentry','postedit','revoke','undoedit'):
        reset();token=ready_topology();preserved=snapshot()
        values=dict(Action='arm',Mode=mode,Token=token,Document=doc,Service=service,UserText='TOPOLOGY USER EVENT')
        hooks.execute(tuple(arg('NamedValue',k,v) for k,v in values.items()))
        try:answer=call('accept',Token=token,Replacement='Joined')
        finally:event=json.loads(hooks.execute((arg('NamedValue','Action','disarm'),)))
        assert event['fired'],event
        if mode in ('edit','postedit','undoedit'):
            record(mode+'-preserves-user-edit',doc.Text.String=='TOPOLOGY USER EVENT' and answer['status']==('REJECTED' if mode=='edit' else 'UNKNOWN_CHANGED'))
        elif mode=='reentry':record('nested-busy',event['nested']['status']=='BUSY' and answer['status']=='APPLIED')
        else:record(mode+'-no-write',answer['status']=='REJECTED' and snapshot()==preserved)
    reset()
    doc.Text.insertControlCharacter(paragraphs()[1].End,LINE_BREAK,False)
    preserved=snapshot();answer=ready_topology(required=False)
    record('soft-line-break-refused-before-write',answer['status']=='REJECTED' and snapshot()==preserved)
    reset()
    choose(2,0,0).ParaAdjust=1
    token=ready_topology();preserved=snapshot()
    answer=call('accept',Token=token,Replacement='Joined')
    record('different-paragraph-styles-no-merge',answer['status']=='REJECTED' and snapshot()==preserved)
    reset()
    token=ready_topology();assert call('accept',Token=token,Replacement='Joined')['status']=='APPLIED'
    manager=doc.getUndoManager();manager.lock()
    try:choose(0,0,3).String='USER'
    finally:manager.unlock()
    preserved=snapshot();error_message=None
    try:manager.undo()
    except Exception as error:error_message=str(error)
    record('user-edit-blocks-topology-undo',error_message is not None and 'no replacement' in error_message and snapshot()==preserved)
    reset()
    file=Path(r'C:\GrammarOutput\topology-synthetic.docx')
    url=uno.systemPathToFileUrl(str(file))
    doc.storeAsURL(url,(arg('PropertyValue','FilterName','Office Open XML Text'),))
    keeper=desktop.loadComponentFromURL('private:factory/swriter','_blank',0,())
    before=snapshot();doc.close(True)
    doc=desktop.loadComponentFromURL(url,'_blank',0,(arg('PropertyValue','MacroExecutionMode',NEVER_EXECUTE),arg('PropertyValue','UpdateDocMode',0)))
    keeper.close(True)
    record('reopened-exact',snapshot()==before)
    digest=hashlib.sha256(file.read_bytes()).hexdigest()
    for i in range(4):
        exercise('Merged 한국어 😀','saved-merge-'+str(i))
        exercise('First\n\nLast','saved-split-'+str(i))
    record('no-automatic-save',hashlib.sha256(file.read_bytes()).hexdigest()==digest)
    result['status']='PASS'

except Exception as error:
    result.update(status='FAIL', error=type(error).__name__ + ': ' + str(error))
    try:
        diagnostic=[]
        for para in paragraphs():
            parts=para.createEnumeration()
            while parts.hasMoreElements():
                part=parts.nextElement()
                diagnostic.append(dict(text=part.String, color=part.CharColor,
                    theme=part.CharColorTheme, tint=part.CharColorTintOrShade, transparency=part.CharTransparence,
                    bag=[(item.Name,str(item.Value)) for item in part.CharInteropGrabBag]))
        result['formatFailureDiagnostic']=diagnostic
    except Exception as diagnostic_error:
        result['diagnosticError']=str(diagnostic_error)
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
