// Save writes the view caret into document metadata. Compare all serialized bytes
// against the recorded caret, then restore the current caret synchronously.
// No content/format exclusions, no await, no event dispatch or snapshot rollback.
export function historyImage(doc,format,hash,recordedCaret=null){
 if(!doc||!['hwp','hwpx'].includes(format))throw Error('GRAMMAR_FORMAT_UNAVAILABLE');
 const read=()=>{const c=JSON.parse(doc.getStoredCaret());if(!c||Object.keys(c).sort().join(',')!=='list,para,pos'||!['list','para','pos'].every(k=>Number.isSafeInteger(c[k])&&c[k]>=0))throw Error('GRAMMAR_CARET_UNAVAILABLE');return c;};
 const current=read(),caret=recordedCaret??current;
 if(!['list','para','pos'].every(k=>Number.isSafeInteger(caret[k])&&caret[k]>=0))throw Error('GRAMMAR_CARET_UNAVAILABLE');
 const resolved=JSON.parse(doc.getCaretPosition());
 if(resolved.sectionIndex!==current.list||resolved.paragraphIndex!==current.para||resolved.charOffset!==current.pos)throw Error('GRAMMAR_CARET_UNAVAILABLE');
 const changed=JSON.stringify(current)!==JSON.stringify(caret);
 try{if(changed)doc.setCaretPosition(caret.list,caret.para,caret.pos);return {caret,hash:hash(format==='hwp'?doc.exportHwp():doc.exportHwpx())};}
 finally{if(changed){doc.setCaretPosition(current.list,current.para,current.pos);if(JSON.stringify(read())!==JSON.stringify(current))throw Error('GRAMMAR_CARET_RESTORE_UNCERTAIN');}}
}
