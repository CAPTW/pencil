import {overlayBootstrap} from './bootstrap-overlay.mjs';
export function overlayProduct(source){
 source=overlayBootstrap(source);
 const edits=[
 ["import { installReview } from '/grammar-review/bootstrap.mjs';", "import { installNativeReview } from '/grammar-review/native-review.mjs';\nimport { inspectDocument } from '/grammar-review/load-protection.mjs';\nimport '/grammar-core.js';\nconst {HOST: grammarNativeHost} = await import(/* @vite-ignore */ chrome.runtime.getURL('host-config.js'));\nlet grammarApprovedGeneration = -1;let grammarParsedGeneration = -1;const grammarLoadProofs=new WeakMap<object,number>();let grammarMountReview: (()=>void)|null=null;"],
 ['grammarReviewDispose=installReview({host:', 'grammarReviewDispose=installNativeReview({runtime:chrome.runtime,location:window.location,nativeHost:grammarNativeHost,Core:globalThis.GrammarCore,host:'],
 ['eligible:()=>false});', "eligible:()=>editMode === 'normal' && grammarApprovedGeneration === wasm.documentGeneration});"],
 ['async function loadDocumentForOpen(data: Uint8Array, fileName: string): Promise<DocumentInfo> {', 'async function loadDocumentForOpen(data: Uint8Array, fileName: string): Promise<DocumentInfo> {\n  grammarApprovedGeneration=-1;grammarParsedGeneration=-1;grammarReviewDispose?.();\n  const grammarChecked=await inspectDocument(data);'],
 ['    return wasm.loadDocument(data, fileName);', "    const info=wasm.loadDocument(data, fileName);\n    if(info.encrypted!==false || info.hwp3Variant===true || wasm.getSourceFormat()!==grammarChecked.format)throw Error('PENCIL_PROTECTION_REFUSED');\n    grammarParsedGeneration=wasm.documentGeneration;grammarLoadProofs.set(info,grammarParsedGeneration);return info;"],
 ["    let grammarHost = document.getElementById('grammar-review-host');", "    grammarMountReview=()=>{grammarReviewDispose?.();\n    let grammarHost = document.getElementById('grammar-review-host');"],
 ["eligible:()=>editMode === 'normal' && grammarApprovedGeneration === wasm.documentGeneration});", "eligible:()=>editMode === 'normal' && grammarApprovedGeneration === wasm.documentGeneration});\n    };grammarMountReview();"],
 ["      suppressDialogs: options.suppressDialogs,\n    });\n  });\n}", "      suppressDialogs: options.suppressDialogs,\n    });\n    if(grammarLoadGeneration===grammarParsedGeneration&&grammarLoadGeneration===wasm.documentGeneration){grammarApprovedGeneration=grammarParsedGeneration;grammarMountReview?.();}\n  });\n}"],
 ['    const docInfo = await loadDocumentForOpen(data, fileName);', '    const docInfo = await loadDocumentForOpen(data, fileName);\n    const grammarLoadGeneration=grammarLoadProofs.get(docInfo);'],
 ['isRecoveryBlocked: () => wasm.requiresPasswordForSave,','isRecoveryBlocked: () => true,'],
 ['recoveryEnabled: settings.recoveryEnabled,','recoveryEnabled: false,'],
 ['idleEnabled: settings.idleSaveEnabled,','idleEnabled: false,'],
 ["import { addRecentDoc, listRecentDocs } from '@/recent/recent-store';",'const addRecentDoc=async (..._args:any[])=>{};const listRecentDocs=async()=>[];']
 ];
 for(const [a,b] of edits){if(source.split(a).length!==2)throw Error('RHWP_PRODUCT_SOURCE_DRIFT: '+a);source=source.replace(a,b);}
 return source;
}
