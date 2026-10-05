"""Sandbox-only public UNO driver for the actual Writer transaction core."""
import os
import json
from grammar_writer_bridge import WriterBridge
import unohelper
from grammar_writer_core import WriterCore


class Writer(WriterCore):
    def __init__(self, context):
        if os.environ.get('USERNAME') != 'WDAGUtilityAccount' or os.environ.get('GRAMMAR_WRITER_FIXTURE') != '1':
            raise RuntimeError('isolated synthetic fixture only')
        super().__init__(context)
        self.bridge = None

    def execute(self, arguments):
        args = {item.Name: item.Value for item in arguments}
        if args.get('Action') == 'start_native_bridge':
            if self.bridge and not self.bridge.finished.is_set():
                return json.dumps(dict(status='BUSY'))
            self.bridge = WriterBridge(self, r'C:\GrammarPayload\writer-native\target-apply.dll')
            self.bridge.start()
            return json.dumps(dict(status='STARTED'))
        if args.get('Action') == 'native_bridge_state':
            return json.dumps(dict(status='STATE', finished=bool(self.bridge and self.bridge.finished.is_set()),
                cleanupComplete=bool(self.bridge and self.bridge.cleanup_complete),
                error=self.bridge.error if self.bridge else None))
        if args.get('Action') == 'stop_native_bridge':
            return json.dumps(self.bridge.stop()) if self.bridge else json.dumps(dict(status='ABSENT'))
        return super().execute(arguments)


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(Writer, 'org.grammar.fixture.Writer', ('org.grammar.fixture.Writer',))
