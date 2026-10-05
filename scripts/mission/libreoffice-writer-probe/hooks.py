"""Adversarial in-process test listeners; never part of a product package."""
import json
import os
import time
import uno
import unohelper
from com.sun.star.task import XJob
from com.sun.star.document import XUndoManagerListener
from com.sun.star.util import XModifyListener


class Hook(unohelper.Base, XUndoManagerListener, XModifyListener):
    def __init__(self, args):
        self.args = args
        self.data = dict(fired=False, nested=None, at=None)

    def fire(self, event_kind):
        if self.args is None:
            return
        mode = self.args['Mode']
        if self.data['fired'] or (mode == 'postedit' and event_kind != 'leave') or (mode == 'modify' and event_kind != 'modify') or (mode == 'undoedit' and event_kind != 'undo'):
            return
        if mode not in ('postedit', 'modify', 'undoedit') and event_kind != 'enter':
            return
        self.data.update(fired=True, at=time.monotonic())
        try:
            if mode in ('edit', 'postedit', 'modify', 'undoedit'):
                self.args['Document'].Text.String = self.args['UserText']
            elif mode in ('delay', 'tickdelay'):
                time.sleep(2.7 if mode == 'delay' else .35)
            else:
                values = dict(Action='disable' if mode == 'revoke' else 'accept',
                    Token=self.args['Token'], Replacement='NESTED WRONG', Deadline=time.monotonic()+2.5)
                arguments = []
                for name, value in values.items():
                    item = uno.createUnoStruct('com.sun.star.beans.NamedValue')
                    item.Name, item.Value = name, value
                    arguments.append(item)
                self.data['nested'] = json.loads(self.args['Service'].execute(tuple(arguments)))
        except Exception as error:
            self.data['hookError'] = type(error).__name__ + ': ' + str(error)

    def enteredContext(self, event):
        self.fire('enter')

    def leftContext(self, event):
        self.fire('leave')

    def modified(self, event):
        self.fire('modify')

    def ignore(self, event):
        pass

    def undoActionAdded(self, event):
        self.fire('undo')

    actionUndone = actionRedone = allActionsCleared = ignore
    redoActionsCleared = resetAll = enteredHiddenContext = leftHiddenContext = cancelledContext = disposing = ignore


class Hooks(unohelper.Base, XJob):
    def __init__(self, context):
        if os.environ.get('USERNAME') != 'WDAGUtilityAccount' or os.environ.get('GRAMMAR_WRITER_FIXTURE') != '1':
            raise RuntimeError('isolated synthetic fixture only')
        self.hook = None

    def execute(self, arguments):
        args = {arg.Name: arg.Value for arg in arguments}
        if args['Action'] == 'arm':
            if self.hook:
                raise RuntimeError('hook already active')
            self.hook = Hook(args)
            if args['Mode'] == 'modify':
                args['Document'].addModifyListener(self.hook)
            else:
                args['Document'].getUndoManager().addUndoManagerListener(self.hook)
            return json.dumps(dict(status='ARMED'))
        if args['Action'] == 'disarm' and self.hook:
            hook, self.hook = self.hook, None
            if hook.args['Mode'] == 'modify':
                hook.args['Document'].removeModifyListener(hook)
            else:
                hook.args['Document'].getUndoManager().removeUndoManagerListener(hook)
            hook.args = None
            return json.dumps(hook.data)
        return json.dumps(dict(status='REJECTED'))


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(Hooks, 'org.grammar.fixture.Hooks', ('org.grammar.fixture.Hooks',))
