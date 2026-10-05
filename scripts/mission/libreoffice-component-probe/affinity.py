"""Synthetic-fixture service only. Not installed in a normal profile or product package."""
import ctypes
import json
import os
import threading
import time
import unohelper
from com.sun.star.task import XJob
from com.sun.star.awt import XCallback
from com.sun.star.lang.SystemDependent import SYSTEM_WIN32


class Callback(unohelper.Base, XCallback):
    def __init__(self, owner, document):
        self.owner, self.document = owner, document

    def notify(self, ignored):
        owner = self.owner
        result = dict(pid=os.getpid(), thread=threading.get_native_id(), status='FAIL')
        try:
            if time.monotonic() > owner.deadline:
                raise RuntimeError('expired before document access')
            window = self.document.CurrentController.Frame.ContainerWindow
            hwnd = int(window.getWindowHandle((), SYSTEM_WIN32))
            pid = ctypes.c_ulong()
            api = ctypes.windll.user32.GetWindowThreadProcessId
            api.argtypes = (ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong))
            api.restype = ctypes.c_ulong
            tid = api(ctypes.c_void_p(hwnd), ctypes.byref(pid))
            result.update(windowPid=pid.value, windowThread=tid)
            if pid.value != os.getpid() or tid != threading.get_native_id():
                raise RuntimeError('not target window thread')
            selection = self.document.CurrentController.getSelection()
            if selection.getCount() != 1:
                raise RuntimeError('not one range')
            # Synthetic fixture string only. No generic monitoring or mutation entry point.
            result['selected'] = selection.getByIndex(0).String
            result['status'] = 'PASS'
        except Exception as error:
            result['error'] = type(error).__name__ + ': ' + str(error)
        owner.result = result
        self.document = None


class Affinity(unohelper.Base, XJob):
    def __init__(self, context):
        if os.environ.get('USERNAME') != 'WDAGUtilityAccount':
            raise RuntimeError('synthetic guest only')
        self.context = context
        self.result = None
        self.used = False
        self.deadline = 0

    def execute(self, arguments):
        args = {arg.Name: arg.Value for arg in arguments}
        if args.get('Action') == 'poll':
            return json.dumps(self.result or dict(status='PENDING'))
        if self.used or args.get('Action') != 'inspect':
            return json.dumps(dict(status='REJECTED'))
        self.used = True
        self.deadline = time.monotonic() + 3
        callback = self.context.ServiceManager.createInstanceWithContext('com.sun.star.awt.AsyncCallback', self.context)
        callback.addCallback(Callback(self, args['Document']), None)
        return json.dumps(dict(status='QUEUED', pid=os.getpid(), callerThread=threading.get_native_id()))


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(Affinity, 'org.grammar.fixture.Affinity', ('org.grammar.fixture.Affinity',))
