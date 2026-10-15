// Strict bounded format admission, before the editor parses any document text.
// Unknown flags/containers are refused rather than treated as unprotected.
const fail=()=>{throw Error('DOCUMENT_PROTECTION_UNKNOWN_OR_UNSUPPORTED');};
const ascii=b=>new TextDecoder('utf-8',{fatal:true}).decode(b);
export async function inspectDocument(bytes){
 if(!(bytes instanceof Uint8Array)||bytes.length<64||bytes.length>8*1024*1024)fail();
 const v=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength),u16=o=>v.getUint16(o,true),u32=o=>v.getUint32(o,true);
 if(bytes.slice(0,8).every((x,i)=>x===[208,207,17,224,161,177,26,225][i])){
  const size=2**u16(30);if(![512,4096].includes(size)||u16(32)!==6||u16(28)!==65534||u32(56)!==4096||u32(72)!==0)fail();
  const sector=id=>{if(!Number.isInteger(id)||id<0||(id+2)*size>bytes.length)fail();return bytes.subarray((id+1)*size,(id+2)*size);};
  const fat=[];const n=u32(44);if(n<1||n>109)fail();
  for(let i=0;i<n;i++){const b=sector(u32(76+4*i)),d=new DataView(b.buffer,b.byteOffset,b.length);for(let j=0;j<b.length;j+=4)fat.push(d.getUint32(j,true));}
  const chain=(start,table,read,max)=>{let id=start,out=[],seen=new Set();while(id!==0xfffffffe){if(seen.has(id)||id>=table.length||out.length>=max)fail();seen.add(id);out.push(read(id));id=table[id];}const joined=new Uint8Array(out.reduce((n,b)=>n+b.length,0));let pos=0;for(const b of out){joined.set(b,pos);pos+=b.length;}return joined;};
  const dir=chain(u32(48),fat,sector,256),d=new DataView(dir.buffer);const entry=i=>{if(i<0||i*128+128>dir.length)fail();const o=i*128,len=d.getUint16(o+64,true);if(len<2||len>64||len%2)fail();return {name:new TextDecoder('utf-16le',{fatal:true}).decode(dir.subarray(o,o+len-2)),type:dir[o+66],left:d.getUint32(o+68,true),right:d.getUint32(o+72,true),child:d.getUint32(o+76,true),start:d.getUint32(o+116,true),length:Number(d.getBigUint64(o+120,true))};};
  const root=entry(0);if(root.type!==5||root.length>bytes.length)fail();let found=null,seen=new Set();
  const visit=i=>{if(i===0xffffffff)return;if(seen.has(i)||seen.size>1024)fail();seen.add(i);const e=entry(i);visit(e.left);if(e.name==='FileHeader'){if(found||e.type!==2||e.length!==256)fail();found=e;}visit(e.right);};visit(root.child);if(!found)fail();
  const miniBytes=chain(u32(60),fat,sector,128),mini=new DataView(miniBytes.buffer),miniFat=[];for(let i=0;i<miniBytes.length;i+=4)miniFat.push(mini.getUint32(i,true));
  const store=chain(root.start,fat,sector,16384).subarray(0,root.length);
  const header=chain(found.start,miniFat,i=>{if(i*64+64>store.length)fail();return store.subarray(i*64,i*64+64);},4);
  if(header.length!==256||ascii(header.subarray(0,17))!=='HWP Document File')fail();const h=new DataView(header.buffer);
  if(header[35]!==5||(h.getUint32(36,true)&~1)!==0)fail();
  return Object.freeze({format:'hwp',protection:'plain'});
 }
 if(u32(0)!==0x04034b50)fail();
 // No ZIP64, multi-disk, encryption, duplicate paths or ambiguous local headers.
 let end=bytes.length-22;while(end>=Math.max(0,bytes.length-65557)&&u32(end)!==0x06054b50)end--;if(end<0||u32(end)!==0x06054b50||u16(end+4)||u16(end+6)||u16(end+8)!==u16(end+10)||end+22+u16(end+20)!==bytes.length)fail();
 const count=u16(end+10),offset=u32(end+16);if(count<1||count>1024||offset+u32(end+12)!==end)fail();let pos=offset,total=0,manifest=null,mime=null;const names=new Set();
 for(let i=0;i<count;i++){
  if(u32(pos)!==0x02014b50)fail();const flags=u16(pos+8),method=u16(pos+10),packed=u32(pos+20),length=u32(pos+24),nl=u16(pos+28),ex=u16(pos+30),cl=u16(pos+32),local=u32(pos+42);
  const name=ascii(bytes.subarray(pos+46,pos+46+nl));total+=length;
  if(flags&~0x808||![0,8].includes(method)||length>16*1024*1024||total>32*1024*1024||names.has(name)||name.includes('..')||name.includes('\\')||name.startsWith('/')||/encrypt|drm|signature|scripts/i.test(name))fail();names.add(name);
  if(u32(local)!==0x04034b50||u16(local+6)!==flags||u16(local+8)!==method)fail();const start=local+30+u16(local+26)+u16(local+28);if(start+packed>offset||ascii(bytes.subarray(local+30,local+30+u16(local+26)))!==name)fail();
  if(name==='mimetype'||name==='META-INF/manifest.xml'){
   if(length>65536||packed>65536)fail();let raw=bytes.slice(start,start+packed);
   if(method===8){const reader=new Blob([raw]).stream().pipeThrough(new DecompressionStream('deflate-raw')).getReader();const parts=[];let size=0;try{while(true){const {value,done}=await reader.read();if(done)break;size+=value.length;if(size>65536)fail();parts.push(value);}}finally{await reader.cancel();}raw=new Uint8Array(size);let p=0;for(const b of parts){raw.set(b,p);p+=b.length;}}
   if(raw.length!==length)fail();if(name==='mimetype')mime=ascii(raw);else manifest=ascii(raw);
  }
  pos+=46+nl+ex+cl;if(pos>end)fail();
 }
 if(pos!==end||mime!=='application/hwp+zip'||!manifest||/<!DOCTYPE|<!ENTITY/i.test(manifest))fail();
 const xml=new DOMParser().parseFromString(manifest,'application/xml'),root=xml.documentElement;
 if(xml.querySelector('parsererror')||root.localName!=='manifest'||root.namespaceURI!=='urn:oasis:names:tc:opendocument:xmlns:manifest:1.0'||root.children.length!==0)fail();
 return Object.freeze({format:'hwpx',protection:'plain'});
}
