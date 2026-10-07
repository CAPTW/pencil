// Task-owned pinned rHWP source server; loopback only. No installation.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,join,posix} from 'node:path';
import {pathToFileURL} from 'node:url';
import {overlayInput,overlayHistory} from '../../adapters/rhwp/input-overlay.mjs';
import {overlayBootstrap} from '../../adapters/rhwp/bootstrap-overlay.mjs';
const root=resolve(process.argv[2]??'');
assert.ok(process.argv[2],'external evidence root required');
const pin=JSON.parse(await readFile(new URL('../../adapters/rhwp/upstream.json',import.meta.url),'utf8'));
const proof=JSON.parse(await readFile(join(root,'upstream-receipt.json'),'utf8'));
assert.equal(proof.sourceCommit,pin.commit);
process.env.RHWP_DISABLE_EXTERNAL_WEBFONTS='1';
process.env.RHWP_WITHOUT_HWPCTRL='1';
const studio=join(root,'upstream/rhwp-studio');
const {createServer}=await import(pathToFileURL(join(studio,'node_modules/vite/dist/node/index.js')).href);
const wasmPath=join(root,'upstream/pkg/rhwp_bg.wasm');
const wasm=await readFile(wasmPath);
const gate=await readFile(new URL('../../adapters/rhwp/accept-gate.mjs',import.meta.url));
const reviewModules=new Map();
for(const name of ['accept-gate','review-session','review-panel','instant-draft','bootstrap','document-admission'])reviewModules.set('/grammar-review/'+name+'.mjs',await readFile(new URL('../../adapters/rhwp/'+name+'.mjs',import.meta.url)));
reviewModules.set('/grammar-core.js',await readFile(new URL('../../adapters/chromium/extension/core.js',import.meta.url)));
const route='/@fs/'+wasmPath.replaceAll('\\','/');
await writeFile(join(studio,'grammar-owned-harness.html'),'<!doctype html><html><body><div id="host" style="height:95vh"></div></body></html>',{flag:'wx'}).catch(e=>{if(e.code!=='EEXIST')throw e;});
const server=await createServer({root:studio,server:{host:'127.0.0.1',port:17707,strictPort:true,hmr:false},optimizeDeps:{noDiscovery:true},plugins:[{
 name:'grammar-input-overlay',enforce:'pre',
 resolveId(id,importer){
  const candidate=id.startsWith('.')&&importer?.startsWith('\0grammar:')?posix.resolve(posix.dirname(importer.slice(9)),id):id;
  if(reviewModules.has(candidate))return '\0grammar:'+candidate;
 },
 load(id){if(id.startsWith('\0grammar:'))return reviewModules.get(id.slice(9))?.toString('utf8');},
 transform(source,id){
  if((process.argv.includes('--bootstrap-fixture')||process.argv.includes('--bootstrap-denied'))&&id.replaceAll('\\','/').endsWith('/src/main.ts'))return overlayBootstrap(source.replaceAll('\r\n','\n'),{syntheticFixture:process.argv.includes('--bootstrap-fixture')});
  if(id.replaceAll('\\','/').endsWith('/src/engine/input-handler.ts'))return overlayInput(source.replaceAll('\r\n','\n'));
  if(id.replaceAll('\\','/').endsWith('/src/engine/history.ts'))return overlayHistory(source.replaceAll('\r\n','\n'));
 }
},{
 name:'owned-wasm-route',configureServer(s){s.middlewares.use((req,res,next)=>{
  if(req.url==='/grammar-accept-gate.mjs'){res.setHeader('Content-Type','text/javascript');return res.end(gate);}
  if(reviewModules.has(req.url)){res.setHeader('Content-Type','text/javascript');return res.end(reviewModules.get(req.url));}
  if(req.url?.split('?')[0]!==route)return next();
  res.setHeader('Content-Type','application/wasm');res.end(wasm);
 });}
}]});
await server.listen();console.log('OWNED_SERVER',process.pid);
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,async()=>{await server.close();process.exit(0);});
