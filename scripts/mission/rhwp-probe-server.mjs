// Task-owned pinned rHWP source server; loopback only. No installation.
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
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
const route='/@fs/'+wasmPath.replaceAll('\\','/');
await writeFile(join(studio,'grammar-owned-harness.html'),'<!doctype html><html><body><div id="host" style="height:95vh"></div></body></html>',{flag:'wx'}).catch(e=>{if(e.code!=='EEXIST')throw e;});
const server=await createServer({root:studio,server:{host:'127.0.0.1',port:17707,strictPort:true,hmr:false},optimizeDeps:{noDiscovery:true},plugins:[{
 name:'owned-wasm-route',configureServer(s){s.middlewares.use((req,res,next)=>{
  if(req.url?.split('?')[0]!==route)return next();
  res.setHeader('Content-Type','application/wasm');res.end(wasm);
 });}
}]});
await server.listen();console.log('OWNED_SERVER',process.pid);
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,async()=>{await server.close();process.exit(0);});
