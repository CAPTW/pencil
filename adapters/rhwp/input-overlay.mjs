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
  const edits=[
    ['export class CommandHistory {', `export class CommandHistory {
  private grammarImages = new WeakMap<EditCommand, {before: string; after: string}>();
  private grammarImage(wasm: WasmBridge): string {
    const doc = wasm.borrowDocumentHandle();
    if (!doc) throw new Error('GRAMMAR_DOCUMENT_UNAVAILABLE');
    const format = wasm.getSourceFormat();
    if (format !== 'hwp' && format !== 'hwpx') throw new Error('GRAMMAR_FORMAT_UNAVAILABLE');
    return grammarHex(grammarSha256(format === 'hwp' ? doc.exportHwp() : doc.exportHwpx()));
  }
  private checkGrammarHistory(command: EditCommand, wasm: WasmBridge, side: 'before' | 'after'): void {
    if (command.type !== 'snapshot:grammar:apply') return;
    const images = this.grammarImages.get(command);
    if (!images || images[side] !== this.grammarImage(wasm)) throw new Error('GRAMMAR_HISTORY_SOURCE_CHANGED');
  }`],
    ['      cursorAfter = command.execute(wasm);\n    } catch (error)', `      const before = command.type === 'snapshot:grammar:apply' ? this.grammarImage(wasm) : null;
      cursorAfter = command.execute(wasm);
      if (before !== null) {
        try { this.grammarImages.set(command, {before, after: this.grammarImage(wasm)}); }
        catch (error) { command.discard?.(wasm); throw error; }
      }
    } catch (error)`],
    ['    const command = this.undoStack[this.undoStack.length - 1];\n    if (!command) return null;', `    const command = this.undoStack[this.undoStack.length - 1];
    if (!command) return null;
    this.checkGrammarHistory(command, wasm, 'after');`],
    ['    const command = this.redoStack[this.redoStack.length - 1];\n    if (!command) return null;', `    const command = this.redoStack[this.redoStack.length - 1];
    if (!command) return null;
    this.checkGrammarHistory(command, wasm, 'before');`]
  ];
  for(const [before,after] of edits){
    if(source.split(before).length!==2)throw new Error('RHWP_HISTORY_FENCE_SOURCE_DRIFT');
    source=source.replace(before,after);
  }
  source="import {sha256 as grammarSha256} from '@noble/hashes/sha2.js';\nimport {bytesToHex as grammarHex} from '@noble/hashes/utils.js';\n"+source;
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
