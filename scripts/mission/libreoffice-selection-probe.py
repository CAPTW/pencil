"""Guest-only real UNO capability probe. Not a Grammar Apply implementation.

All edits are fixture setup or explicit test-driver actions on owned documents.
No claim of atomic multi-call UNO validation/mutation is made.
"""
import json
import os
from pathlib import Path
import sys
import time

if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
    raise RuntimeError('Guest only')

import uno
import unohelper
from com.sun.star.util import XModifyListener
from com.sun.star.document.MacroExecMode import NEVER_EXECUTE


class Revision(unohelper.Base, XModifyListener):
    def __init__(self):
        self.value = 0

    def modified(self, event):
        self.value += 1

    def disposing(self, event):
        pass


result = dict(status='RUNNING', scope='real GUI UNO capability, not Grammar Accept', checks=[],
              impressMethod=sys.argv[2] if len(sys.argv) > 2 else 'direct')
output = Path(r'C:\GrammarOutput\uno-result.json')
documents = []
desktop = None


def save():
    output.write_text(json.dumps(result, indent=2), encoding='utf-8')


def check(name, condition):
    if not condition:
        raise AssertionError(name)
    result['checks'].append(name)
    save()


def prop(name, value):
    p = uno.createUnoStruct('com.sun.star.beans.PropertyValue')
    p.Name, p.Value = name, value
    return p


def create(kind):
    doc = desktop.loadComponentFromURL('private:factory/' + kind, '_blank', 0,
        (prop('Hidden', False), prop('MacroExecutionMode', NEVER_EXECUTE)))
    documents.append(doc)
    check(kind + '-visible-controller', doc.getCurrentController() is not None
          and doc.CurrentController.Frame.ContainerWindow.isVisible())
    return doc


try:
    save()
    local = uno.getComponentContext()
    resolver = local.ServiceManager.createInstanceWithContext('com.sun.star.bridge.UnoUrlResolver', local)
    end = time.monotonic() + 30
    while True:
        try:
            context = resolver.resolve('uno:pipe,name=' + sys.argv[1] + ';urp;StarOffice.ComponentContext')
            break
        except Exception:
            if time.monotonic() >= end:
                raise
            time.sleep(.1)
    desktop = context.ServiceManager.createInstanceWithContext('com.sun.star.frame.Desktop', context)
    source = 'Before | Synthetic 한국어 😀 | After'
    replacement = 'Reviewed 한국어 문장 😀'
    writer = create('swriter')
    writer.Text.String = source
    revision = Revision()
    writer.addModifyListener(revision)
    try:
        cursor = writer.Text.createTextCursor()
        cursor.gotoStart(False)
        cursor.goRight(9, False)
        cursor.goRight(9, True)  # Select ASCII 'Synthetic', avoid UTF-16 ambiguity.
        controller = writer.getCurrentController()
        controller.select(cursor)
        selected = controller.getSelection().getByIndex(0)
        check('writer-exact-selection', selected.String == 'Synthetic')
        check('writer-range-identity', writer.Text.compareRegionStarts(selected, cursor) == 0
              and writer.Text.compareRegionEnds(selected, cursor) == 0)
        before_revision = revision.value
        elsewhere = writer.Text.createTextCursor()
        elsewhere.gotoEnd(False)
        controller.select(elsewhere)
        moved = controller.getSelection().getByIndex(0)
        check('writer-selection-move-observable', writer.Text.compareRegionStarts(moved, cursor) != 0)
        check('writer-selection-move-no-text-change', writer.Text.String == source)
        controller.select(cursor)
        undo = writer.getUndoManager()
        undo.clear()
        undo.enterUndoContext('Grammar synthetic capability probe')
        try:
            cursor.String = replacement
        finally:
            undo.leaveUndoContext()
        check('writer-exact-replacement', writer.Text.String == source.replace('Synthetic', replacement))
        check('writer-modification-event', revision.value > before_revision)
        check('writer-undo-available', undo.isUndoPossible())
        undo.undo()
        check('writer-undo-exact', writer.Text.String == source)
        # An equal string elsewhere is not the captured target range.
        tail = writer.Text.createTextCursor()
        tail.gotoEnd(False)
        tail.String = ' Synthetic'
        duplicate = writer.Text.createTextCursor()
        duplicate.gotoEnd(False)
        duplicate.goLeft(9, True)
        original = writer.Text.createTextCursor()
        original.gotoStart(False)
        original.goRight(9, False)
        original.goRight(9, True)
        check('writer-duplicate-source-distinct-range', duplicate.String == original.String
              and writer.Text.compareRegionStarts(duplicate, original) != 0)
        captured_revision = revision.value
        original.String = 'USER EDIT'
        check('writer-user-edit-invalidates-source-and-revision', original.String != 'Synthetic'
              and revision.value > captured_revision)
    finally:
        writer.removeModifyListener(revision)

    calc = create('scalc')
    sheet = calc.Sheets.getByIndex(0)
    cell = sheet.getCellByPosition(1, 2)
    neighbor = sheet.getCellByPosition(2, 2)
    cell.String, neighbor.String = source, 'KEEP NEIGHBOR'
    calc.CurrentController.select(cell)
    current = calc.CurrentController.getSelection()
    address = current.getRangeAddress()
    check('calc-cell-binding', address.StartColumn == address.EndColumn == 1
          and address.StartRow == address.EndRow == 2 and current.String == source)
    undo = calc.getUndoManager()
    undo.clear()
    undo.enterUndoContext('Grammar synthetic capability probe')
    try:
        cell.String = replacement
    finally:
        undo.leaveUndoContext()
    check('calc-exact-replacement', cell.String == replacement and neighbor.String == 'KEEP NEIGHBOR')
    check('calc-undo-available', undo.isUndoPossible())
    undo.undo()
    check('calc-undo-exact', cell.String == source and neighbor.String == 'KEEP NEIGHBOR')

    impress = create('simpress')
    page = impress.DrawPages.getByIndex(0)
    shape = impress.createInstance('com.sun.star.drawing.TextShape')
    page.add(shape)
    shape.setSize(uno.createUnoStruct('com.sun.star.awt.Size', 15000, 3000))
    shape.String = source
    impress.CurrentController.select(shape)
    shapes = impress.CurrentController.getSelection()
    check('impress-shape-binding', shapes.getCount() == 1 and shapes.getByIndex(0) == shape
          and shapes.getByIndex(0).String == source)
    undo = impress.getUndoManager()
    undo.clear()
    undo.enterUndoContext('Grammar synthetic capability probe')
    try:
        if len(sys.argv) > 2 and sys.argv[2] == 'dispatch':
            dispatcher = context.ServiceManager.createInstanceWithContext('com.sun.star.frame.DispatchHelper', context)
            frame = impress.CurrentController.Frame
            dispatcher.executeDispatch(frame, '.uno:Text', '', 0, ())
            dispatcher.executeDispatch(frame, '.uno:SelectAll', '', 0, ())
            dispatcher.executeDispatch(frame, '.uno:InsertText', '', 0, (prop('Text', replacement),))
        else:
            shape.String = replacement
    finally:
        undo.leaveUndoContext()
    check('impress-exact-replacement', shape.String == replacement)
    check('impress-undo-available', undo.isUndoPossible())
    undo.undo()
    check('impress-undo-exact', shape.String == source)
    result['status'] = 'PASS'
except Exception as error:
    result['status'] = 'FAIL'
    result['error'] = type(error).__name__ + ': ' + str(error)
    save()  # Preserve original failure even if the application bridge dies during cleanup.
finally:
    closed = 0
    for document in reversed(documents):
        try:
            document.setModified(False)
            document.close(True)
            closed += 1
        except Exception as error:
            result['closeError'] = str(error)
    result['closedDocuments'] = closed
    result['createdDocuments'] = len(documents)
    try:
        result['desktopTerminated'] = bool(desktop.terminate()) if desktop else False
    except Exception as error:
        result['desktopTerminated'] = False
        result['terminationError'] = str(error)
        result['status'] = 'FAIL'
    if closed != len(documents):
        result['status'] = 'FAIL'
    save()
sys.exit(0 if result['status'] == 'PASS' else 1)
