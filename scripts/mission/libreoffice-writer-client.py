"""Real Writer transaction fixture client; invoke only through guarded Sandbox launcher."""
import json
import os
from pathlib import Path
import sys
import time
from types import SimpleNamespace
if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
    raise RuntimeError('Guest only')
import uno
from com.sun.star.document.MacroExecMode import NEVER_EXECUTE

out = Path(r'C:\GrammarOutput\uno-result.json')
result = dict(status='RUNNING', scope='experimental in-process Writer transaction', eventHarness='target_internal', checks=[])
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
    enabled = call('enable', Document=doc)
    assert enabled['status'] == 'ENABLED', enabled
    captured = call('capture')
    assert captured['status'] == 'CAPTURED' and captured['source'] == 'Synthetic', captured
    return captured['token']




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
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Writer', ctx)
    assert service is not None
    hooks = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Hooks', ctx)
    assert hooks is not None
    record('disabled-capture-refused', call('capture')['status'] == 'REJECTED')
    token = capture()
    record('capture-does-not-change-document', doc.Text.String == source)
    replacement = 'Reviewed 한국어 😀'
    reply = call('accept', Token=token, Replacement=replacement)
    record('reviewed-replacement-exact', reply['status'] == 'APPLIED'
        and doc.Text.String == source.replace('Synthetic', replacement))
    record('modified-flag-and-notifications-restored', doc.isModified() and doc.isSetModifiedEnabled())
    after = doc.Text.String
    record('replay-refused-unchanged', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == after)
    doc.getUndoManager().undo()
    record('undo-original-exact', doc.Text.String == source)

    token = capture()
    select(0, 6)
    record('moved-selection-refused', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source)

    token = capture()
    select(0, 6)
    select()
    record('selection-away-and-back-refused', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source)

    token = capture()
    changed = source.replace('Synthetic', 'USER EDIT')
    doc.Text.String = changed
    record('user-edit-preserved', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == changed)

    token = capture()
    doc.Text.String = source + ' USER TAIL'
    doc.Text.String = source
    doc.setModified(False)
    select()
    record('source-away-and-back-revision-refused', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source)

    token = capture()
    record('expired-operation-refused', call('accept', Token=token, Replacement='WRONG', Deadline=0.0)['status'] == 'REJECTED'
        and doc.Text.String == source)
    record('dismiss-clears-authority', call('dismiss')['status'] == 'DISMISSED'
        and call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED' and doc.Text.String == source)

    token = capture()
    call('disable')
    record('disable-revokes-authority', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source)

    token = capture()
    other = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0,
        (arg('PropertyValue', 'Hidden', False), arg('PropertyValue', 'MacroExecutionMode', NEVER_EXECUTE)))
    try:
        other.Text.String = 'OTHER DOCUMENT'
        reply = call('accept', Token=token, Replacement='WRONG')
        record('different-document-refused', reply['status'] == 'REJECTED'
            and other.Text.String == 'OTHER DOCUMENT' and doc.Text.String == source)
    finally:
        other.setModified(False)
        other.close(True)
    doc.CurrentController.Frame.activate()
    for mode in ('edit', 'delay', 'reentry', 'postedit', 'revoke'):
        token = capture()
        user_text = source + ' USER EVENT' if mode == 'edit' else 'USER POSTWRITE EVENT'
        hook_args = dict(Action='arm', Mode=mode, Token=token, Document=doc, Service=service, UserText=user_text)
        hooks.execute(tuple(arg('NamedValue', k, v) for k, v in hook_args.items()))
        try:
            reply = call('accept', Token=token, Replacement=replacement)
        finally:
            hook = SimpleNamespace(**json.loads(hooks.execute((arg('NamedValue', 'Action', 'disarm'),))))
        if mode == 'edit':
            record('reentrant-user-edit-preserved', hook.fired and reply['status'] == 'REJECTED'
                and doc.Text.String == source + ' USER EVENT')
        elif mode == 'delay':
            record('original-deadline-not-restarted-after-callback', hook.fired and reply['status'] == 'REJECTED'
                and reply['reason'] == 'expired_operation' and doc.Text.String == source)
        elif mode == 'reentry':
            record('nested-accept-busy-single-write', hook.fired and hook.nested['status'] == 'BUSY'
                and reply['status'] == 'APPLIED' and doc.Text.String == source.replace('Synthetic', replacement))
        elif mode == 'postedit':
            record('postwrite-user-edit-preserved-with-uncertain-outcome', hook.fired
                and reply['status'] == 'UNKNOWN_CHANGED' and doc.Text.String == 'USER POSTWRITE EVENT')
        else:
            result['revocation'] = dict(nested=hook.nested, outer=reply, unchanged=doc.Text.String == source)
            observe_failure('inflight-permission-revocation-prevents-write', hook.fired
                and hook.nested['status'] == 'REVOKING' and reply['status'] == 'REJECTED'
                and doc.Text.String == source and not call('state')['enabled'])
    token = capture()
    hook_args = dict(Action='arm', Mode='modify', Token=token, Document=doc, Service=service, UserText='USER INSIDE MODIFY CALLBACK')
    hooks.execute(tuple(arg('NamedValue', k, v) for k, v in hook_args.items()))
    try:
        reply = call('accept', Token=token, Replacement=replacement)
    finally:
        hook = SimpleNamespace(**json.loads(hooks.execute((arg('NamedValue', 'Action', 'disarm'),))))
    result['modifyRace'] = dict(fired=hook.fired, callbackAt=hook.at, reply=reply, finalText=doc.Text.String)
    observe_failure('mutation-callback-user-edit-preserved', hook.fired and reply['status'] == 'UNKNOWN_CHANGED'
        and doc.Text.String == 'USER INSIDE MODIFY CALLBACK')

    token = capture()
    hook_args = dict(Action='arm', Mode='undoedit', Token=token, Document=doc, Service=service, UserText='USER INSIDE UNDO CALLBACK')
    hooks.execute(tuple(arg('NamedValue', k, v) for k, v in hook_args.items()))
    try:
        reply = call('accept', Token=token, Replacement=replacement)
    finally:
        hook = SimpleNamespace(**json.loads(hooks.execute((arg('NamedValue', 'Action', 'disarm'),))))
    result['undoRace'] = dict(fired=hook.fired, callbackAt=hook.at, reply=reply, finalText=doc.Text.String)
    observe_failure('undo-callback-user-edit-preserved', hook.fired and reply['status'] == 'UNKNOWN_CHANGED'
        and doc.Text.String == 'USER INSIDE UNDO CALLBACK')

    token = capture()
    reads = call('state')['textReads']
    call('capture', Deadline=0.0)
    record('expired-capture-no-text-read', call('state')['textReads'] == reads)
    call('disable')
    doc.disableSetModified()
    try:
        denied = call('enable', Document=doc)
        record('foreign-notification-batch-not-overridden', denied['status'] == 'REJECTED'
            and not doc.isSetModifiedEnabled() and doc.Text.String == source)
    finally:
        doc.enableSetModified()
    section = doc.createInstance('com.sun.star.text.TextSection')
    section.IsProtected = True
    cursor = doc.Text.createTextCursor()
    doc.Text.insertTextContent(cursor, section, False)
    original = doc.Text.String
    reads = call('state')['textReads']
    denied = call('enable', Document=doc)
    record('protected-section-refused-before-text-read', denied['status'] == 'REJECTED'
        and call('state')['textReads'] == reads and not call('state')['enabled'] and doc.Text.String == original)
    doc.Text.removeTextContent(section)
    token = capture()
    time.sleep(30.1)
    record('expired-review-cache-cleared', not call('state')['reviewRetained'] and not call('state')['resultRetained'])
    record('real-review-expiry-refused', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED'
        and doc.Text.String == source)
    token = capture()
    doc.setModified(False)
    doc.close(True)
    doc = None
    state = call('state')
    record('document-close-clears-review-and-binding', not state['enabled'] and not state['reviewRetained'])
    record('closed-document-accept-refused', call('accept', Token=token, Replacement='WRONG')['status'] == 'REJECTED')
    result['status'] = 'FAIL' if result.get('failures') else 'PASS'
except Exception as error:
    result.update(status='FAIL', error=type(error).__name__ + ': ' + str(error))
finally:
    try:
        if service:
            result['disable'] = call('disable')['status']
        if doc:
            doc.setModified(False)
            doc.close(True)
        result['desktopTerminated'] = bool(desktop.terminate()) if desktop else False
    except Exception as error:
        result.update(status='FAIL', cleanupError=str(error))
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')
sys.exit(0 if result['status'] == 'PASS' else 1)
