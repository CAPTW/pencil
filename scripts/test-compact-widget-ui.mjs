// Production App, mocked Tauri commands/events. No credentials or external calls.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile, mkdir} from 'node:fs/promises';
import path from 'node:path';
const require=createRequire(import.meta.url);
const {build}=require('esbuild');
const {chromium}=require(path.resolve(process.argv[2]));
const out=path.resolve(process.argv[3]); await mkdir(out,{recursive:true});
const mock=`export const invoke=async(c,a)=>window.fixtureInvoke(c,a);
export const listen=async(n,f)=>{window.listeners[n]=f;return ()=>delete window.listeners[n];};
export const getCurrentWindow=()=>({setSize:async s=>{window.sizes.push(s);},startDragging:async()=>{}});
export class LogicalSize {constructor(width,height){this.width=width;this.height=height;}}`;
const bundle=await build({stdin:{contents:`import React from 'react';import {createRoot} from 'react-dom/client';import App from './src/App';createRoot(document.getElementById('root')).render(<App/>);`,resolveDir:process.cwd(),loader:'tsx'},bundle:true,write:false,format:'iife',jsx:'automatic',plugins:[{name:'mock',setup(b){b.onResolve({filter:/^@tauri-apps\/api\//},()=>({path:'mock',namespace:'mock'}));b.onLoad({filter:/.*/,namespace:'mock'},()=>({contents:mock,loader:'js'}));}}]});
const browser=await chromium.launch({headless:true,chromiumSandbox:true,executablePath:process.argv[4]});
try {
 const page=await browser.newPage({viewport:{width:380,height:360}});
 let network=0;await page.route(/^https?:/,r=>{network++;return r.abort();});
 await page.setContent('<div id="root"></div>');await page.addStyleTag({content:await readFile('src/styles.css','utf8')});
 await page.evaluate(()=>{window.listeners={};window.sizes=[];window.fixtureInvoke=async c=>{
   if(c==='local_model_status')return {phase:'unmanaged'};
   if(c==='take_startup_notices')return [];
   if(c==='check_prerequisites')return {commands:[]};
   if(c==='auth_status')return {loggedIn:false};
   if(c==='api_session_status')return {epoch:'synthetic',configured:false,consent:false};
   return null;
 };});
 await page.addScriptTag({content:bundle.outputFiles[0].text});
 await page.waitForFunction(()=>window.listeners['selection-captured']);
 await page.evaluate(()=>window.listeners['selection-captured']({payload:{sessionId:'owned',generation:1,selectedText:'Synthetic source.',directApply:true,acceptExpiresAtMs:Date.now()+30000}}));
 await page.evaluate(()=>window.listeners['instant-candidate']({payload:{sessionId:'owned',generation:1,candidateText:'Synthetic suggestion.'}}));
 const draft=page.getByRole('textbox',{name:'Editable rewrite result text'});
 await draft.waitFor();assert.equal(await draft.inputValue(),'Synthetic suggestion.');
 assert.equal(await page.getByRole('button',{name:'Details',exact:true}).getAttribute('aria-expanded'),'false');
 assert.equal(await page.locator('.mode-grid').isVisible(),false);
 for(const testid of ['run-api','apply-result','copy-result']){
   const box=await page.getByTestId(testid).boundingBox();assert.ok(box && box.y>=0 && box.y+box.height<=360, testid+' visible');
 }
 await page.evaluate(()=>{const previous=window.fixtureInvoke;window.commands=[];window.fixtureInvoke=async(c,a)=>{
   window.commands.push(c);
   if(c==='save_settings')return a.settings;
   if(c==='api_session_status')return {epoch:'synthetic',endpoint:'https://api.example.invalid/v1/messages',model:'synthetic-model',requestEnabled:true};
   return previous(c,a);
 };});
 await page.getByTestId('run-api').click();
 const confirmation=page.getByRole('dialog',{name:'Send to API?'});
 await confirmation.waitFor();
 for(const name of ['Cancel','Send request']){
   const box=await confirmation.getByRole('button',{name,exact:true}).boundingBox();
   assert.ok(box && box.x>=0 && box.y>=0 && box.x+box.width<=380 && box.y+box.height<=360, name+' fits compact window');
 }
 assert.equal(await confirmation.getByRole('button',{name:'Cancel',exact:true}).evaluate(el=>el===document.activeElement),true);
 await page.keyboard.press('Escape');
 await confirmation.waitFor({state:'detached'});
 assert.equal(await page.evaluate(()=>window.commands.some(c=>c.includes('rewrite'))),false,'Cancel does not infer');
 await page.getByTestId('run-api').click();
 await confirmation.waitFor();
 await page.evaluate(()=>window.listeners['selection-captured']({payload:{sessionId:'owned-new',generation:2,selectedText:'New synthetic source.',directApply:true,acceptExpiresAtMs:Date.now()+30000}}));
 await confirmation.getByRole('button',{name:'Send request',exact:true}).click();
 await confirmation.waitFor({state:'detached'});
 assert.equal(await page.evaluate(()=>window.commands.some(c=>c.includes('rewrite'))),false,'Stale confirmation does not infer');
 await page.evaluate(()=>window.listeners['instant-candidate']({payload:{sessionId:'owned-new',generation:2,candidateText:'New synthetic suggestion.'}}));
 await page.waitForFunction(()=>document.querySelector('[aria-label="Editable rewrite result text"]').value==='New synthetic suggestion.');
 await draft.fill('Owner edited draft.');
 await page.getByRole('button',{name:'Details',exact:true}).click();
 assert.equal(await page.locator('.mode-grid').isVisible(),true);
 assert.equal(await draft.inputValue(),'Owner edited draft.');
 await page.getByRole('button',{name:'Simple',exact:true}).click();
 assert.equal(await draft.inputValue(),'Owner edited draft.');
 assert.deepEqual(await page.evaluate(()=>window.sizes.at(-1)),{width:380,height:360});
 await page.screenshot({path:path.join(out,'compact.png')});
 page.once('dialog',dialog=>dialog.accept());
 await page.getByRole('combobox',{name:'Writing action'}).selectOption('translate');
 await page.waitForFunction(()=>window.sizes.at(-1)?.height===400);
 await page.setViewportSize({width:380,height:400});
 const resultBox=await draft.boundingBox();
 const contentBox=await page.locator('.panel-content').boundingBox();
 assert.ok(resultBox && contentBox && resultBox.y>=contentBox.y && resultBox.y+resultBox.height<=contentBox.y+contentBox.height,'Translation draft fits above fixed footer without scrolling');
 await page.screenshot({path:path.join(out,'compact-translate.png')});
 await page.getByRole('button',{name:'Settings',exact:true}).click();
 await page.getByRole('heading',{name:'Connect a model'}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Connect Codex',exact:true}).isVisible(),false);
 await page.getByText('Account login / CLI connections',{exact:true}).click();
 await page.getByRole('button',{name:'Connect Codex',exact:true}).waitFor();
 assert.equal(network,0);
 console.log('PASS: compact default, fixed actions, editable draft survives Details toggle, resize request, collapsed CLI, zero network (mock Tauri, not installed)');
} finally {await browser.close();}
