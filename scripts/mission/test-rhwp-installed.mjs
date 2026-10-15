import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
// Run only inside the owned single-session supervisor with an installed package.
const config=JSON.parse(await readFile(process.argv[2],'utf8'));
const r=config.evidenceRoot;
const {chromium}=createRequire(import.meta.url)(config.playwrightModule);
const manifest=JSON.parse(await readFile(r+'/install/MANIFEST.json','utf8'));
const ext=r+'/install/extension',origin='chrome-extension://'+manifest.extensionId;
const core=await import(pathToFileURL(config.wasmModule));
await core.default({module_or_path:await readFile(config.wasmBinary)});
const snapshot=async path=>{const d=new core.HwpDocument(await readFile(path));try{return Array.from({length:d.getSectionCount()},(_,s)=>Array.from({length:d.getParagraphCount(s)},(_,p)=>({text:d.getTextRange(s,p,0,d.getParagraphLength(s,p)),chars:Array.from({length:d.getParagraphLength(s,p)},(_,i)=>JSON.parse(d.getCharPropertiesAt(s,p,i))),para:JSON.parse(d.getParaPropertiesAt(s,p)),style:JSON.parse(d.getStyleAt(s,p))}))).flat();}finally{d.free();}};
let browser,page;const result={status:'FAIL',scope:'INSTALLED_NATIVE_MESSAGING_NO_ANALYSIS_BRIDGE',checks:[],samples:[],errors:[],externalRequests:[]};
try{
 browser=await chromium.launchPersistentContext(config.browserProfile,{executablePath:config.chromiumBinary,headless:true,chromiumSandbox:true,acceptDownloads:true,ignoreDefaultArgs:['--disable-extensions'],args:['--disable-extensions-except='+ext,'--load-extension='+ext]});
 browser.on('request',q=>{if(/^https?:/.test(q.url()))result.externalRequests.push(new URL(q.url()).origin);});
 page=await browser.newPage();page.on('pageerror',e=>result.errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')console.log('PAGE_ERROR',m.text().slice(0,300));});page.on('dialog',d=>d.accept());
 await page.goto(origin+'/rhwp/index.html');await page.locator('#grammar-review-host').waitFor();console.log('PANEL_READY');
 assert.equal(await page.evaluate(()=>typeof __wasm),'undefined');assert.equal(await page.evaluate(()=>typeof grammarLocalAnalyze),'undefined');
 for(const format of ['hwp','hwpx'])for(const mode of ['ko','paraphrase','polite']){
  const input=config.inputs[format];
  const before=await snapshot(input);const source=before[1].text;console.log('OPEN',format,mode,source.length);
  const oldGeneration=await page.locator('#grammar-review-host').getAttribute('data-grammar-generation');
  await page.locator('#file-input').setInputFiles(input);
  await page.waitForFunction(old=>!!document.querySelector('#grammar-review-host')&&!document.documentElement.classList.contains('rhwp-busy')&&document.querySelector('#grammar-review-host').dataset.grammarGeneration!==old,oldGeneration);
  await page.getByRole('button',{name:'Enable review',exact:true}).waitFor();
  await page.getByLabel('Local writing mode').selectOption(mode);
  await page.getByRole('button',{name:'Enable review',exact:true}).click();
  await page.locator('#scroll-container canvas').first().click({position:{x:120,y:120}});const inputBox=page.getByLabel('문서 편집 입력');await inputBox.focus();await inputBox.press('Control+Home');await inputBox.press('Control+ArrowDown');await inputBox.press('Home');
  for(let i=0;i<Array.from(source).length;i++)await inputBox.press('Shift+ArrowRight');
  const start=performance.now();await page.getByRole('button',{name:'Capture selection',exact:true}).click();
  console.log('CAPTURE_STATUS',await page.locator('#grammar-review-host [role=status]').textContent());
  await page.locator('#grammar-review-host [role=status]').filter({hasText:/^Local Instant ready/}).waitFor({timeout:15000});
  const text=await page.getByLabel('Reviewed replacement').inputValue();assert.match(text,/25/);if(mode==='ko')assert.match(text,/[가-힣]/);
  const sample={format,mode,source,output:text,recommendationMs:performance.now()-start};
  const t=performance.now();await page.getByRole('button',{name:'Accept',exact:true}).click();await page.locator('#grammar-review-host [role=status]').filter({hasText:/^Applied$/}).waitFor();sample.acceptMs=performance.now()-t;
  const save=async suffix=>{const download=page.waitForEvent('download',{timeout:15000});await inputBox.focus();await inputBox.press('Control+s');const d=await download;const path=r+'/'+format+'-'+mode+'-'+suffix+'.'+format;await d.saveAs(path);return path;};
  const saved=await save('accepted');const after=await snapshot(saved);assert.equal(after[1].text,text);assert.deepEqual(after[0],before[0]);assert.deepEqual(after[2],before[2]);assert.deepEqual(after[1].para,before[1].para);assert.deepEqual(after[1].style,before[1].style);for(const c of after[1].chars)assert.deepEqual(c,before[1].chars[0]);result.checks.push(format+' '+mode+' Accept exact saved text+format');
  await inputBox.focus();await inputBox.press('Control+z');const undone=await save('undo');assert.deepEqual(await snapshot(undone),before);result.checks.push(format+' '+mode+' Undo exact saved text+format');
  const savedGeneration=await page.locator('#grammar-review-host').getAttribute('data-grammar-generation');await page.locator('#file-input').setInputFiles(saved);await page.waitForFunction(old=>!!document.querySelector('#grammar-review-host')&&!document.documentElement.classList.contains('rhwp-busy')&&document.querySelector('#grammar-review-host').dataset.grammarGeneration!==old,savedGeneration);const reopen=await save('reopened');assert.deepEqual(await snapshot(reopen),after);result.checks.push(format+' '+mode+' reopen exact text+format');result.samples.push(sample);
 }
 for(const format of ['hwp','hwpx']){
  await page.locator('#file-input').setInputFiles(r+'/protected-synthetic.'+format);
  await page.locator('#sb-message').filter({hasText:'DOCUMENT_PROTECTION_UNKNOWN_OR_UNSUPPORTED'}).waitFor();
  assert.equal(await page.getByRole('button',{name:'Accept',exact:true}).count(),0);
  result.checks.push(format+' protected original bytes refused before editor admission');
 }
 assert.deepEqual(result.externalRequests,[]);result.checks.push('no HTTP document or model bridge requests');
 result.status='PASS';
}catch(e){result.error=String(e.stack??e);if(page)result.visible=await page.locator('body').innerText().catch(()=> 'unavailable');}
finally{if(browser)await browser.close();await writeFile(r+'/installed-receipt.json',JSON.stringify(result,null,2));}
console.log(result.status,result.checks.length);process.exitCode=result.status==='PASS'?0:1;
