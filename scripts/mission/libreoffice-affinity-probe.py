"""Run with the guarded guest launcher and the task-owned fixture OXT only."""
import json
import os
from pathlib import Path
import sys
import time
if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
    raise RuntimeError('Guest only')
import uno
from com.sun.star.document.MacroExecMode import NEVER_EXECUTE

out = Path(r'C:\GrammarOutput\uno-result.json')
result = dict(status='RUNNING', scope='in-process UI affinity only', checks=[])
desktop = doc = None


def arg(type_name, name, value):
    p = uno.createUnoStruct('com.sun.star.beans.' + type_name)
    p.Name, p.Value = name, value
    return p


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
    doc.Text.String = 'Before Synthetic After'
    cursor = doc.Text.createTextCursor()
    cursor.goRight(7, False)
    cursor.goRight(9, True)
    doc.CurrentController.select(cursor)
    service = ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Affinity', ctx)
    assert service is not None, 'fixture component not loaded'
    response = json.loads(service.execute((arg('NamedValue', 'Action', 'inspect'), arg('NamedValue', 'Document', doc))))
    assert response['status'] == 'QUEUED'
    assert response['pid'] != os.getpid(), 'service accidentally running in external test client'
    result['checks'].append('component-in-separate-target-process')
    end = time.monotonic() + 5
    while True:
        inspected = json.loads(service.execute((arg('NamedValue', 'Action', 'poll'),)))
        if inspected['status'] != 'PENDING':
            break
        if time.monotonic() >= end:
            raise AssertionError('callback deadline')
        time.sleep(.02)
    result['callback'] = inspected
    assert inspected['status'] == 'PASS', inspected
    assert inspected['pid'] == inspected['windowPid'] == response['pid']
    assert inspected['thread'] == inspected['windowThread']
    result['checks'].append('callback-on-real-writer-window-thread')
    assert inspected['selected'] == 'Synthetic'
    assert doc.Text.String == 'Before Synthetic After'
    result['checks'].append('exact-selection-read-document-unchanged')
    replay = json.loads(service.execute((arg('NamedValue', 'Action', 'inspect'), arg('NamedValue', 'Document', doc))))
    assert replay['status'] == 'REJECTED'
    result['checks'].append('fixture-one-shot-replay-refused')
    result['status'] = 'PASS'
except Exception as error:
    result.update(status='FAIL', error=type(error).__name__ + ': ' + str(error))
finally:
    try:
        if doc:
            doc.setModified(False)
            doc.close(True)
        result['desktopTerminated'] = bool(desktop.terminate()) if desktop else False
    except Exception as error:
        result.update(status='FAIL', cleanupError=str(error))
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')
sys.exit(0 if result['status'] == 'PASS' else 1)
