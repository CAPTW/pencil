// Production component and styles, mocked callbacks. No Provider or user profile.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFile} from 'node:fs/promises';
import path from 'node:path';
const require=createRequire(import.meta.url);
const {build}=require('esbuild');
const {chromium}=require(path.resolve(process.argv[2]));
const bundle=await build({stdin:{contents:`import React from 'react';import {createRoot} from 'react-dom/client';import {AcceptBar} from './src/AcceptBar';const root=createRoot(document.getElementById('root'));window.renderBar=(expiry,key='capture',direct=true)=>root.render(<main className="shell"><section className="panel"><div className="panel-content"><div style={{height:1800,flexShrink:0}}>Long provider explanation and draft</div></div><AcceptBar key={key} directApply={direct} expiresAtMs={expiry} canCopy={true} busy={false} canRunApi={true} onRunApi={()=>window.apis++} onAccept={()=>window.accepts++} onCopy={()=>window.copies++}/></section></main>);`,resolveDir:process.cwd(),loader:'tsx'},bundle:true,write:false,format:'iife',jsx:'automatic'});
const browser=await chromium.launch({headless:true,chromiumSandbox:true,executablePath:process.argv[3]});
try {
 const page=await browser.newPage({viewport:{width:420,height:460}});
 await page.setContent('<div id="root"></div>');await page.addStyleTag({content:await readFile('src/styles.css','utf8')});
 await page.evaluate(()=>{window.accepts=0;window.copies=0;window.apis=0;});await page.addScriptTag({content:bundle.outputFiles[0].text});
 await page.evaluate(()=>window.renderBar(Date.now()+30000));
 const accept=page.getByTestId('apply-result');await accept.waitFor();
 for(const size of [{width:420,height:460},{width:320,height:360}]) {
  await page.setViewportSize(size);
  for(const top of [0,1800]) {
   await page.locator('.panel-content').evaluate((el,top)=>el.scrollTop=top,top);
   for (const button of [accept,page.getByTestId("run-api")]) {const box=await button.boundingBox();assert(box && box.y>=0 && box.y+box.height<=size.height);}
  }
 }
 await page.getByTestId("run-api").click();assert.equal(await page.evaluate(()=>window.apis),1);
 await accept.click();assert.equal(await page.evaluate(()=>window.accepts),1);
 await page.evaluate(()=>window.renderBar(Date.now()+600,'short'));
 await page.waitForTimeout(900);await page.getByText('Accept expired — select and capture again, or Copy.',{exact:true}).waitFor();
 assert.equal(await accept.isDisabled(),true);
 await page.getByTestId('copy-result').click();assert.equal(await page.evaluate(()=>window.copies),1);
 // A late result/render cannot renew the same capture's monotonic deadline.
 await page.evaluate(()=>window.renderBar(Date.now()+30000,'short'));assert.equal(await accept.isDisabled(),true);
 await page.evaluate(()=>window.renderBar(undefined,'legacy'));await page.waitForTimeout(50);assert.equal(await accept.isDisabled(),true);
 await page.evaluate(()=>window.renderBar(Date.now()+30000,'recaptured'));await page.waitForTimeout(50);assert.equal(await accept.isEnabled(),true);
 await accept.focus();await page.keyboard.press('Enter');assert.equal(await page.evaluate(()=>window.accepts),2);
 await page.evaluate(()=>window.renderBar(Date.now()+30000,'copy-only',false));
 await page.waitForTimeout(50);assert.equal(await accept.count(),0);assert.equal(await page.getByTestId('copy-result').isEnabled(),true);
 console.log('PASS: fixed actions at 2 sizes/2 scroll positions; explicit and keyboard Accept; expiry; Copy after expiry; no render renewal; missing deadline refuses; recapture resets');
} finally {await browser.close();}
