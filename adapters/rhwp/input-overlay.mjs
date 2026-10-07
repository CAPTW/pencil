// Narrow source overlay for the pinned rHWP InputHandler. Fail on source drift.
// No change to non-Grammar operations. Not installed in the shipped product.
export function overlayInput(source) {
  const replacements=[
    ['  async executeDocumentAgentOperation(', '  get grammarNoRenderRollback() { return true; }\n\n  async executeDocumentAgentOperation('],
    ['    } catch (renderError) {\n      try {\n        const restored = this.history.rollbackUncommittedSnapshot',
     "    } catch (renderError) {\n      if (desc.operationType === 'grammar:apply') {\n        this.history.discardFailedGrammarSnapshot(command, this.wasm);\n        try { this.eventBus.emit('document-mutated', 'grammar-uncertain'); } catch {}\n        try { this.eventBus.emit('document-changed', 'grammar-uncertain'); } catch {}\n        throw new DocumentAgentRenderCommitError(renderError, false);\n      }\n      try {\n        const restored = this.history.rollbackUncommittedSnapshot"]
  ];
  for(const [before,after] of replacements){
    if(source.split(before).length!==2)throw new Error('RHWP_OVERLAY_SOURCE_DRIFT');
    source=source.replace(before,after);
  }
  return source;
}

export function overlayHistory(source) {
  const marker='  rollbackUncommittedSnapshot(';
  if(source.split(marker).length!==2)throw new Error('RHWP_HISTORY_SOURCE_DRIFT');
  return source.replace(marker,`  discardFailedGrammarSnapshot(command: EditCommand, wasm: WasmBridge): void {
    // Remove only this exact failed command; never undo or remove later edits.
    for (const stack of [this.undoStack, this.redoStack]) {
      const index = stack.indexOf(command);
      if (index >= 0) stack.splice(index, 1);
    }
    command.discard?.(wasm);
  }

`+marker);
}
