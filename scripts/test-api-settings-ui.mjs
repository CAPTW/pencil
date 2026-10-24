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
   if(command==='api_session_status')return {...window.fixtureStatus};
   if(command==='clear_api_session'){window.fixtureStatus=empty('cleared-'+Math.random());return {...window.fixtureStatus};}
   if(command==='configure_api_session'){
    const s=args.setup;if(s.expectedEpoch!==window.fixtureStatus.epoch)throw Error('stale');
    const next={epoch:'saved-'+Math.random(),configured:true,endpoint:s.endpoint,model:s.model,consent:s.consent,requestEnabled:false};
    if(window.fixtureDelayed)return new Promise(resolve=>{window.fixtureResolve=()=>resolve(next);});
    window.fixtureStatus=next;return {...next};
   }throw Error('unexpected fixture command');
  };
 });
 await page.addScriptTag({content:bundle.outputFiles[0].text});
 const save=page.getByRole('button',{name:'Keep setup for this session'});
 await save.waitFor();await page.waitForFunction(()=>!document.querySelector('button').disabled);
 for (const endpoint of ['https://api.anthropic.com/v1/messages','https://generativelanguage.googleapis.com/v1beta/openai/chat/completions','https://openrouter.ai/api/v1/chat/completions','https://api.x.ai/v1/chat/completions']) {
  await page.getByLabel('API key (session only)').fill('previous-provider-key');
  await page.getByRole('checkbox').check();
  await page.getByLabel('API provider preset').selectOption(endpoint);
  assert.equal(await page.getByLabel('HTTPS endpoint').inputValue(),endpoint);
  assert.equal(await page.getByLabel('API key (session only)').inputValue(),'');
  assert.equal(await page.getByRole('checkbox').isChecked(),false);
 }
 checks.push('four provider presets clear draft credentials and permission');
 await page.getByLabel('HTTPS endpoint').fill('https://api.example.test/v1/chat/completions');
 await page.getByLabel('Model',{exact:true}).fill('fixture-model');
 await page.getByLabel('API key (session only)').fill('dummy-secret');await page.getByRole('checkbox').check();await save.click();
 await page.getByText('Setup kept for this app session.',{exact:false}).waitFor();
 assert.equal(await page.getByLabel('API key (session only)').inputValue(),'');checks.push('save clears key input; setup only notice');
 await page.getByRole('button',{name:'Remove API key and permission'}).click();await page.getByText('API key and permission removed from this session.').waitFor();
 assert.equal(await page.getByRole('checkbox').isChecked(),false);checks.push('explicit removal clears permission');
 await page.evaluate(()=>{window.fixtureDelayed=true;});
 await page.getByLabel('API key (session only)').fill('second-dummy');await page.getByRole('checkbox').check();await save.click();
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
