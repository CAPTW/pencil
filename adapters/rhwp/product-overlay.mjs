import {overlayBootstrap} from './bootstrap-overlay.mjs';
export function overlayProduct(source){
 source=overlayBootstrap(source);
 const edits=[
 ["import { installReview } from '/grammar-review/bootstrap.mjs';", "import { installNativeReview } from '/grammar-review/native-review.mjs';\nimport { inspectDocument } from '/grammar-review/load-protection.mjs';\nimport '/grammar-core.js';\nconst {HOST: grammarNativeHost} = await import(/* @vite-ignore */ chrome.runtime.getURL('host-config.js'));\nlet grammarApprovedGeneration = -1;"],
 ['grammarReviewDispose=installReview({host:', 'grammarReviewDispose=installNativeReview({runtime:chrome.runtime,location:window.location,nativeHost:grammarNativeHost,Core:globalThis.GrammarCore,host:'],
 ['eligible:()=>false});', "eligible:()=>editMode === 'normal' && grammarApprovedGeneration === wasm.documentGeneration});"],
 ['async function loadDocumentForOpen(data: Uint8Array, fileName: string): Promise<DocumentInfo> {', 'async function loadDocumentForOpen(data: Uint8Array, fileName: string): Promise<DocumentInfo> {\n  grammarApprovedGeneration=-1;\n  const grammarChecked=await inspectDocument(data);'],
 ['    return wasm.loadDocument(data, fileName);', "    const info=wasm.loadDocument(data, fileName);\n    if(info.encrypted!==false || info.hwp3Variant===true || wasm.getSourceFormat()!==grammarChecked.format)throw Error('PENCIL_PROTECTION_REFUSED');\n    grammarApprovedGeneration=wasm.documentGeneration;return info;"],
 ['isRecoveryBlocked: () => wasm.requiresPasswordForSave,','isRecoveryBlocked: () => true,'],
 ['recoveryEnabled: settings.recoveryEnabled,','recoveryEnabled: false,'],
 ['idleEnabled: settings.idleSaveEnabled,','idleEnabled: false,'],
 ["import { addRecentDoc, listRecentDocs } from '@/recent/recent-store';",'const addRecentDoc=async (..._args:any[])=>{};const listRecentDocs=async()=>[];']
 ];
 for(const [a,b] of edits){if(source.split(a).length!==2)throw Error('RHWP_PRODUCT_SOURCE_DRIFT: '+a);source=source.replace(a,b);}
 return source;
}
