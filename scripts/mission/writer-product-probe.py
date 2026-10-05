"""Owned guest product-entry test. No fixture UNO services or product bypass."""
import ctypes
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import time
if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
    raise RuntimeError('Guest only')
import uno
from com.sun.star.document.MacroExecMode import NEVER_EXECUTE

out = Path(r'C:\GrammarOutput\uno-result.json')
result = dict(status='RUNNING', scope='product Writer menu/accessibility and authenticated native client; not installed widget', checks=[])
desktop = doc = None
source = 'Before | Synthetic 한국어 😀 | After'


def arg(name, value):
    item = uno.createUnoStruct('com.sun.star.beans.PropertyValue')
    item.Name, item.Value = name, value
    return item


def check(name, condition):
    assert condition, name
    result['checks'].append(name)
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')


def wait(probe, seconds=5):
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        found = probe()
        if found:
            return found
        time.sleep(.03)
    raise AssertionError('condition timeout')


def accessible_names():
    found = []
    def walk(item, depth=0):
        if depth > 8 or len(found) > 256:
            return
        context = item.getAccessibleContext()
        found.append((context.getAccessibleName(), context))
        for index in range(min(context.getAccessibleChildCount(), 64)):
            walk(context.getAccessibleChild(index), depth + 1)
    for index in range(toolkit.getTopWindowCount()):
        window = toolkit.getTopWindow(index)
        if window.isVisible():
            accessible = window.getProperty('XAccessible')
            if accessible is None:
                continue
            context = accessible.getAccessibleContext()
            if context.getAccessibleName().startswith('Grammar Writer'):
                walk(accessible)
    return found


def button(name):
    context = wait(lambda: next((c for n, c in accessible_names() if re.fullmatch(re.escape(name) + r'(?:\([A-Z]\))?', n)), None))
    assert context.doAccessibleAction(0)


try:
    out.write_text(json.dumps(result), encoding='utf-8')
    ctx = uno.getComponentContext()
    resolver = ctx.ServiceManager.createInstanceWithContext('com.sun.star.bridge.UnoUrlResolver', ctx)
    def connect():
        try: return resolver.resolve('uno:pipe,name=' + sys.argv[1] + ';urp;StarOffice.ComponentContext')
        except Exception: return None
    ctx = wait(connect, 30)
    desktop = ctx.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', ctx)
    toolkit = ctx.ServiceManager.createInstanceWithContext('com.sun.star.awt.Toolkit', ctx)
    check('fixture-service-not-installed', ctx.ServiceManager.createInstanceWithContext('org.grammar.fixture.Writer', ctx) is None)
    doc = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0,
        (arg('Hidden', False), arg('MacroExecutionMode', NEVER_EXECUTE)))
    wait(lambda: desktop.CurrentComponent == doc)
    doc.Text.String = source
    saved_path = None
    if '--saved' in sys.argv:
        saved_path = Path(r'C:\GrammarOutput\product-owned.docx')
        assert not saved_path.exists()
        url = uno.systemPathToFileUrl(str(saved_path))
        doc.storeAsURL(url, (arg('FilterName', 'Office Open XML Text'), arg('Overwrite', False)))
        # Keep a synthetic document open across reopen so the UNO process lives.
        keeper = desktop.loadComponentFromURL('private:factory/swriter', '_blank', 0, (arg('Hidden', False),))
        doc.close(True)
        doc = desktop.loadComponentFromURL(url, '_blank', 0,
            (arg('Hidden', False), arg('MacroExecutionMode', NEVER_EXECUTE), arg('UpdateDocMode', 0)))
        keeper.close(True)
        wait(lambda: desktop.CurrentComponent == doc)
        check('saved-docx-reopened-exact', doc.Text.String == source and doc.hasLocation())
        saved_digest = hashlib.sha256(saved_path.read_bytes()).hexdigest()
    format_cursor = doc.Text.createTextCursor()
    format_cursor.gotoStart(False)
    format_cursor.gotoEnd(True)
    names = ('CharWeight', 'CharPosture', 'CharHeight', 'CharFontName', 'CharStyleName', 'CharUnderline', 'ParaStyleName')
    result['plainFormatStates'] = {name: format_cursor.getPropertyState(name).value for name in names}
    format_cursor.CharWeight = 150.0
    result['boldFormatStates'] = {name: format_cursor.getPropertyState(name).value for name in names}
    format_cursor.setPropertyToDefault('CharWeight')
    cursor = doc.Text.createTextCursor()
    cursor.gotoStart(False)
    cursor.goRight(9, False)
    cursor.goRight(9, True)
    doc.CurrentController.select(cursor)
    hwnd = int(doc.CurrentController.Frame.ContainerWindow.getWindowHandle((), 1))
    foreground = ctypes.windll.user32.SetForegroundWindow
    foreground.argtypes, foreground.restype = [ctypes.c_void_p], ctypes.c_int
    get_foreground = ctypes.windll.user32.GetForegroundWindow
    get_foreground.restype = ctypes.c_void_p
    def native():
        doc.CurrentController.Frame.activate()
        doc.CurrentController.Frame.ComponentWindow.setFocus()
        focus_ok = foreground(ctypes.c_void_p(hwnd))
        result['focusDiagnostic'] = dict(expected=hwnd, before=get_foreground(), setForeground=bool(focus_ok))
        try:
            wait(lambda: get_foreground() == hwnd)
        finally:
            result['focusDiagnostic']['after'] = get_foreground()
            native_handle = doc.CurrentController.Frame.ContainerWindow.getWindowHandle((), 1)
            result['focusDiagnostic']['currentDocumentHandle'] = int(native_handle)
        completed = subprocess.run([r'C:\GrammarPayload\writer-native\writer-bridge-probe.exe',
            r'C:\GrammarPayload\writer-native\target-apply.dll', str(hwnd)],
            capture_output=True, text=True, timeout=10, creationflags=subprocess.CREATE_NO_WINDOW)
        assert completed.returncode == 0, completed.stderr
        return json.loads(completed.stdout)
    if '--widget' in sys.argv:
        result['scope'] = 'installed Grammar widget through product Writer entry'
        log = open(r'C:\GrammarOutput\widget-driver-error.log', 'w', encoding='utf-8')
        node = subprocess.Popen([r'C:\GrammarPayload\node.exe', r'C:\GrammarPayload\writer-app-e2e.mjs'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log, text=True, encoding='utf-8',
            creationflags=subprocess.CREATE_NO_WINDOW)
        dispatcher = ctx.ServiceManager.createInstanceWithContext('com.sun.star.frame.DispatchHelper', ctx)
        for line in node.stdout:
            try:
                command = json.loads(line)['action']
                value = True
                if command == 'prepare':
                    doc.Text.String = 'Before | please seperate this. | After'
                    cursor = doc.Text.createTextCursor()
                    cursor.gotoStart(False)
                    cursor.goRight(9, False)
                    cursor.goRight(21, True)
                    doc.CurrentController.select(cursor)
                elif command == 'enable':
                    doc.CurrentController.Frame.activate()
                    dispatcher.executeDispatch(doc.CurrentController.Frame, 'service:org.grammar.Writer?review', '', 0, ())
                    button('Enable one review')
                    wait(lambda: any('enabled for one review' in n for n, _ in accessible_names()))
                elif command == 'focus':
                    doc.CurrentController.Frame.activate()
                    doc.CurrentController.Frame.ComponentWindow.setFocus()
                    foreground(ctypes.c_void_p(hwnd))
                    wait(lambda: get_foreground() == hwnd)
                elif command == 'hotkey':
                    assert get_foreground() == hwnd
                    key = ctypes.windll.user32.keybd_event
                    key.argtypes = [ctypes.c_ubyte, ctypes.c_ubyte, ctypes.c_uint32, ctypes.c_size_t]
                    for vk in (0x11, 0x10, 0x47): key(vk, 0, 0, 0)
                    time.sleep(.08)
                    for vk in (0x47, 0x10, 0x11): key(vk, 0, 2, 0)
                elif command == 'read': value = doc.Text.String
                elif command == 'undo': doc.getUndoManager().undo()
                elif command == 'change': doc.Text.String = 'USER SYNTHETIC CHANGE'
                elif command == 'disable': button('Disable')
                elif command == 'settled': wait(lambda: any('Review ended.' in n for n, _ in accessible_names()))
                else: raise ValueError('unrecognized owned fixture action')
                answer = dict(value=value)
            except Exception as error:
                answer = dict(error=type(error).__name__ + ': ' + str(error))
            node.stdin.write(json.dumps(answer) + '\n')
            node.stdin.flush()
        node.wait(timeout=15)
        log.close()
        widget_result = json.loads(Path(r'C:\GrammarOutput\app-writer-result.json').read_text(encoding='utf-8'))
        result['widget'] = widget_result
        check('installed-widget-product-flow', node.returncode == 0 and widget_result['status'] == 'PASS' and widget_result['appNatural'])
    else:
        denied = native()
        check('default-off-no-native-capture', denied['captured'] == 4 and doc.Text.String == source)
        dispatcher = ctx.ServiceManager.createInstanceWithContext('com.sun.star.frame.DispatchHelper', ctx)
        dispatcher.executeDispatch(doc.CurrentController.Frame, 'service:org.grammar.Writer?review', '', 0, ())
        wait(lambda: accessible_names())
        result['initialPanelNames'] = [n for n, _ in accessible_names()]
        check('menu-opens-visible-disabled-panel', any('disabled' in n for n, _ in accessible_names()))
        button('Enable one review')
        wait(lambda: any('enabled for one review' in n for n, _ in accessible_names()))
        check('explicit-visible-document-opt-in', True)
        accepted = native()
        result['nativePeer'] = accepted
        check('product-entry-exact-accept-reread', accepted['captured'] == 0 and accepted['applyCode'] == 0 and accepted['outcome'] == 3
            and doc.Text.String == source.replace('Synthetic', 'Reviewed 한국어 😀'))
        doc.getUndoManager().undo()
        check('product-entry-undo-original', doc.Text.String == source)
        wait(lambda: any('Review ended.' in n for n, _ in accessible_names()))
        check('review-completion-visibly-disabled', True)
        denied = native()
        check('no-silent-rearm', denied['captured'] == 4 and doc.Text.String == source)
        doc.CurrentController.select(cursor)
        button('Enable one review')
        wait(lambda: any('enabled for one review' in n for n, _ in accessible_names()))
        button('Disable')
        wait(lambda: any('disabled' in n for n, _ in accessible_names()))
        denied = native()
        check('explicit-disable-refuses-capture', denied['captured'] == 4 and doc.Text.String == source)
    if saved_path:
        check('product-apply-never-auto-saves-docx', hashlib.sha256(saved_path.read_bytes()).hexdigest() == saved_digest)
    result['status'] = 'PASS'
except Exception as error:
    result.update(status='FAIL', error=type(error).__name__ + ': ' + str(error))
    try: result['panelNames'] = [n for n, _ in accessible_names()]
    except Exception: pass
finally:
    if doc:
        try:
            doc.setModified(False)
            doc.close(True)
        except Exception: pass
    if desktop:
        try: result['desktopTerminated'] = bool(desktop.terminate())
        except Exception: pass
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')
sys.exit(0 if result['status'] == 'PASS' else 1)
