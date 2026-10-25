// Original React component, mocked Tauri boundary; not installed-app acceptance.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir,writeFile} from 'node:fs/promises';
import path from 'node:path';
const require=createRequire(import.meta.url);
const {build}=require('esbuild');
const {chromium}=require(path.resolve(process.argv[2]));
const output=path.resolve(process.argv[3]);await mkdir(output,{recursive:true});
const bundle=await build({stdin:{contents:`import React from 'react';import {createRoot} from 'react-dom/client';import {ApiSettings} from './src/ApiSettings';createRoot(document.getElementById('root')).render(<ApiSettings/>);`,resolveDir:process.cwd(),loader:'tsx'},bundle:true,write:false,format:'iife',jsx:'automatic',plugins:[{name:'tauri-fixture',setup(b){b.onResolve({filter:/^@tauri-apps\/api\/core$/},()=>({path:'fixture',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const invoke=(...args)=>window.fixtureInvoke(...args);',loader:'js'}));}}]});
const browser=await chromium.launch({headless:true,chromiumSandbox:true,executablePath:process.argv[4]});const checks=[];
try{
 const page=await browser.newPage();let network=0;await page.route(/^https?:/,route=>{network++;return route.abort();});
 await page.setContent('<div id="root"></div>');
 await page.evaluate(()=>{
  const empty=epoch=>({epoch,configured:false,endpoint:null,model:null,consent:false,requestEnabled:false});
  window.fixtureStatus=empty('initial');window.fixtureDelayed=false;
  window.fixtureInvoke=async(command,args)=>{
   if(command==='list_api_models') {
    window.catalogArgs=args; window.catalogCalls=(window.catalogCalls??0)+1;
    if(window.catalogError)throw Error('catalog error with secret');
    const rows=[{id:'test-model',name:'Synthetic model',selectable:true},{id:'image-only',name:'Image model',selectable:false}];
    if(window.catalogDelayed)return new Promise(resolve=>{window.catalogResolve=()=>resolve(rows);});
    return rows;
   }
   if(command==='api_session_status')return {...window.fixtureStatus};
   if(command==='clear_api_session'){window.fixtureStatus=empty('cleared-'+Math.random());return {...window.fixtureStatus};}
   if(command==='revoke_api_consent'){window.fixtureStatus={...window.fixtureStatus,consent:false,requestEnabled:false};return {...window.fixtureStatus};}
   if(command==='configure_api_session'){
    const s=args.setup;if(s.expectedEpoch!==window.fixtureStatus.epoch)throw Error('stale');
    const next={epoch:'saved-'+Math.random(),configured:true,endpoint:s.endpoint,model:s.model,consent:s.consent,requestEnabled:false};
    if(window.fixtureDelayed)return new Promise(resolve=>{window.fixtureResolve=()=>resolve(next);});
    window.fixtureStatus=next;return {...next};
   }throw Error('unexpected fixture command');
  };
 });
 await page.addScriptTag({content:bundle.outputFiles[0].text});
 const save=page.getByRole('button',{name:'Save API setup securely'});
 await save.waitFor();await page.waitForFunction(()=>!document.querySelector('button').disabled);
 assert.equal(await page.getByLabel('HTTPS endpoint').isVisible(),false);
 await page.getByText('Endpoint and manual model ID',{exact:true}).click();
 await page.getByText('Privacy and connection management',{exact:true}).click();
 checks.push('advanced endpoint and connection management collapsed by default, explicitly expandable');
 for (const endpoint of ['https://api.anthropic.com/v1/messages','https://generativelanguage.googleapis.com/v1beta/openai/chat/completions','https://openrouter.ai/api/v1/chat/completions','https://api.x.ai/v1/chat/completions']) {
  await page.getByLabel('API key (Windows secure storage)').fill('previous-provider-key');
  await page.getByRole('checkbox').check();
  await page.getByLabel('API provider preset').selectOption(endpoint);
  assert.equal(await page.getByLabel('HTTPS endpoint').inputValue(),endpoint);
  assert.equal(await page.getByLabel('API key (Windows secure storage)').inputValue(),'');
  assert.equal(await page.getByRole('checkbox').isChecked(),false);
 }
 checks.push('four provider presets clear draft credentials and permission');
 const beforeAuto=await page.evaluate(()=>window.catalogCalls??0);
 await page.getByLabel('API key (Windows secure storage)').fill('auto-synthetic-key');
 assert.equal(await page.evaluate(()=>window.catalogCalls??0),beforeAuto);
 await page.getByLabel('Model',{exact:true}).focus();
 await page.waitForFunction(n=>window.catalogCalls===n,beforeAuto+1);
 await page.getByText('Model list loaded: 2.',{exact:false}).waitFor();
 await page.getByLabel('API key (Windows secure storage)').focus();
 await page.getByLabel('Model',{exact:true}).focus();
 assert.equal(await page.evaluate(()=>window.catalogCalls),beforeAuto+1);
 checks.push('completed key entry auto-loads once, not on keystrokes or unchanged refocus');
 await page.getByLabel('API key (Windows secure storage)').fill('do-not-send-to-next-provider');
 const beforeSwitch=await page.evaluate(()=>window.catalogCalls);
 await page.getByLabel('API provider preset').selectOption('https://api.anthropic.com/v1/messages');
 assert.equal(await page.evaluate(()=>window.catalogCalls),beforeSwitch);
 assert.equal(await page.getByLabel('API key (Windows secure storage)').inputValue(),'');
 checks.push('provider switch clears key without automatic old-key request');

 await page.getByLabel('API key (Windows secure storage)').fill('catalog-dummy');
 await page.getByRole('button',{name:'Load model list'}).click();
 await page.getByText('Model list loaded: 2.',{exact:false}).waitFor();
 assert.equal(await page.getByRole('checkbox').isChecked(),false);
 assert.notEqual(await page.getByRole('option',{name:/Image model/}).getAttribute('disabled'),null);
 await page.getByLabel('Available models').selectOption('test-model');
 assert.equal(await page.getByLabel('Model',{exact:true}).inputValue(),'test-model');
 assert.match(await page.locator('body').textContent(),/Actual invocation: not verified/);
 const args=await page.evaluate(()=>window.catalogArgs);assert.deepEqual(Object.keys(args.lookup).sort(),['endpoint','key']);
 checks.push('catalog-only lookup, user selection, unsupported rows disabled, invocation remains unverified');
 await page.getByLabel('API key (Windows secure storage)').fill('changed-key');
 assert.equal(await page.getByLabel('Available models').isDisabled(),true);
 checks.push('key edit invalidates catalog');
 await page.evaluate(()=>{window.catalogError=true;});
 await page.getByRole('button',{name:'Load model list'}).click();
 await page.getByText('Model lookup failed.',{exact:false}).waitFor();
 assert.ok(!(await page.locator('body').textContent()).includes('error with secret'));
 assert.equal(await page.getByLabel('Available models').isDisabled(),true);
 checks.push('failed lookup clears results and suppresses raw errors');
 await page.evaluate(()=>{window.catalogError=false;window.catalogDelayed=true;});
 await page.getByRole('button',{name:'Load model list'}).click();
 await page.waitForFunction(()=>typeof window.catalogResolve==='function');
 await page.evaluate(async()=>{await window.fixtureInvoke('clear_api_session');window.dispatchEvent(new Event('grammar-api-cleared'));window.catalogResolve();});
 await page.waitForFunction(()=>document.body.textContent.includes('Model list not loaded'));
 assert.equal(await page.getByLabel('Available models').isDisabled(),true);
 checks.push('late catalog response cannot restore revoked list');

 await page.getByLabel('HTTPS endpoint').fill('https://api.example.test/v1/chat/completions');
 await page.getByLabel('Model',{exact:true}).fill('fixture-model');
 await page.getByLabel('API key (Windows secure storage)').fill('dummy-secret');await page.getByRole('checkbox').check();await save.click();
 await page.getByText('Setup saved in Windows Credential Manager.',{exact:false}).waitFor();
 assert.equal(await page.getByLabel('API key (Windows secure storage)').inputValue(),'');checks.push('save clears key input; setup only notice');
 await page.evaluate(()=>document.getElementById('root').replaceWith(Object.assign(document.createElement('div'),{id:'root'})));
 const beforeRemount=await page.evaluate(()=>window.catalogCalls);
 await page.addScriptTag({content:bundle.outputFiles[0].text});
 await page.getByText('Endpoint and manual model ID',{exact:true}).click();
 await page.getByText('Privacy and connection management',{exact:true}).click();
 await page.waitForFunction(()=>document.body.textContent.includes('Saved endpoint: https://api.example.test/v1/chat/completions'));
 assert.equal(await page.getByLabel('Model',{exact:true}).inputValue(),'fixture-model');
 assert.equal(await page.getByLabel('API key (Windows secure storage)').inputValue(),'');
 assert.equal(await page.getByRole('checkbox').isChecked(),true);
 assert.equal(await page.evaluate(()=>window.catalogCalls),beforeRemount);
 checks.push('remounted form restores model and consent without returning saved key or fetching catalogs');
 await page.getByRole('button',{name:'Revoke API consent'}).click();await page.getByText('API consent revoked.',{exact:false}).waitFor();assert.equal(await page.getByRole('checkbox').isChecked(),false);assert.equal(await page.getByLabel('Model',{exact:true}).inputValue(),'fixture-model');checks.push('revocation retains selected model and removes consent');
 await page.getByRole('button',{name:'Remove API key and permission'}).click();await page.getByText('Saved API key and permission deleted from Windows Credential Manager.').waitFor();
 assert.equal(await page.getByRole('checkbox').isChecked(),false);checks.push('explicit removal clears permission');
 await page.evaluate(()=>{window.fixtureDelayed=true;});
 await page.getByLabel('API key (Windows secure storage)').fill('second-dummy');await page.getByRole('checkbox').check();await save.click();
 await page.waitForFunction(()=>typeof window.fixtureResolve==='function');
 await page.evaluate(async()=>{await window.fixtureInvoke('clear_api_session');window.dispatchEvent(new Event('grammar-api-cleared'));});
 await page.waitForFunction(()=>document.body.textContent.includes('Permission: not granted'));
 await page.evaluate(()=>window.fixtureResolve());await page.waitForTimeout(50);
 assert.equal(await page.getByRole('checkbox').isChecked(),false);
 assert.match(await page.locator('body').textContent(),/Saved endpoint: None/);checks.push('late save response cannot restore revoked UI state');
 assert.equal(network,0);checks.push('zero external requests');
 await writeFile(path.join(output,'ui-result.json'),JSON.stringify({status:'PASS',classification:'ORIGINAL_REACT_COMPONENT_MOCK_TAURI_NOT_INSTALLED',checks},null,2));
 console.log('PASS',checks.length,'component checks');
}finally{await browser.close();}
