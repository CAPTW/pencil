// Trusted editor metadata only. Necessary refusal guard, not a sensitivity classifier.
// The host must separately grant eligible() for the exact supported document.
export function createDocumentAdmission(wasm, eligible = () => false) {
  let generation = null, denied = false, closed = false;
  return Object.freeze({
    allowed() {
      if (closed) return false;
      try {
        const current = wasm.documentGeneration;
        if (!Number.isSafeInteger(current) || current < 0) { denied = true; return false; }
        if (current !== generation) { generation = current; denied = false; }
        if (denied) return false;
        if (wasm.hasLoadedDocument() !== true || wasm.requiresPasswordForSave !== false ||
            !['hwp', 'hwpx'].includes(wasm.getSourceFormat()) || eligible() !== true) {
          denied = true;
          return false;
        }
        return true;
      } catch {
        denied = true;
        return false;
      }
    },
    close() { closed = true; denied = true; }
  });
}
