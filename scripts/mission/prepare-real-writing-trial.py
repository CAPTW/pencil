"""Prepare an offline, owned Sandbox trial; does not start the Sandbox."""
from pathlib import Path
import shutil
import os
import argparse

parser = argparse.ArgumentParser(__doc__)
parser.add_argument('--local-assets', action='store_true', help='Use production offline preparation inside guest before startup')
parser.add_argument('--resume-preparation', action='store_true', help='Explicitly regenerate owned harness files after interrupted preparation; never use while guest is running')
args = parser.parse_args()

root = Path('D:/dev/grammar-evidence/grammar-real-writing-20261011')
old = Path('D:/dev/grammar-evidence/grammar-writing-package-20261011/payload')
root.mkdir(exist_ok=args.resume_preparation)
p = root / 'payload'
p.mkdir(exist_ok=True)
(root / 'output').mkdir(exist_ok=True)
def owned_link(source, target):
    if not Path(target).exists():
        os.link(source, target)
    return target
for name in ['pwsh', 'node_modules', 'package-pristine']:
    shutil.copytree(old / name, p / name, copy_function=owned_link, dirs_exist_ok=True)
for name in ['node.exe','target-editor.exe','managed-target-focus.exe','target-service-probe.exe','MediumLaunch.exe','host-machine.txt']:
    shutil.copy2(old / name,p / name)
owned_link('D:/dev/grammar-evidence/grammar-gemma-20261007/gemma-4-12b-it-qat-q4_0.gguf',p / 'model.gguf')
s = (Path(__file__).parent / 'managed-app-e2e.mjs').read_text(encoding='utf-8')
s = s.replace("'C:/PencilTrialScope/install/Start-Grammar.ps1'],", "'C:/PencilTrialScope/install/Start-Grammar.ps1','-RuntimeDirectory','C:/GrammarRuntime','-ModelPath','C:/GrammarPayload/model.gguf'],")
s = s.replace("const statusOnly=", "report.classification='INSTALLED_REAL_GEMMA_12B_OFFLINE_SANDBOX';report.samples=[];const trialStart=performance.now();\nconst statusOnly=")
begin = s.index(" if(!statusOnly)await capture();")
end = s.index(" await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('quit_app'))",begin)
s = s[:begin] + '''
 await until('real model ready',async()=>{const state=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('local_model_status'));report.lastModelState=state;if(state.phase==='failed')throw Error('Real model startup failed');return state.phase==='ready';},210000);
 report.readyMs=performance.now()-trialStart;
 for(const [mode,target] of [['natural','en'],['concise','en'],['polite','en'],['translate','ko'],['translate','en']]){
  await page.evaluate(async({mode,target})=>{const s=await window.__TAURI_INTERNALS__.invoke('load_settings');await window.__TAURI_INTERNALS__.invoke('save_settings',{settings:{...s,mode,autoRewrite:false,translation:{...s.translation,targetLanguage:target,applyFormat:'translation_only'}}});},{mode,target});
  await page.reload();await page.waitForFunction(()=>typeof window.__TAURI_INTERNALS__?.transformCallback==='function');
  await page.evaluate(()=>{window.__captures=[];window.__errors=[];window.__analysis=[];const t=window.__TAURI_INTERNALS__;return Promise.all(['selection-captured','capture-error','local-analysis-completed','local-analysis-failed'].map((event,i)=>t.invoke('plugin:event|listen',{event,target:{kind:'Any'},handler:t.transformCallback(e=>(i===0?window.__captures:i===1?window.__errors:window.__analysis).push({event,...e.payload}))})));});
  const sample={mode,target,source:'please seperate this.',status:'FAIL'};report.samples.push(sample);
  try{
   const start=performance.now();await capture();
   await until('real candidate',async()=>{const v=await draft.inputValue();return v.length>0&&v!=='please seperate this.';},18000);
   sample.recommendationMs=performance.now()-start;sample.output=await draft.inputValue();sample.events=await page.evaluate(()=>window.__analysis);
   const edited=sample.output+' Edited.';await draft.fill(edited);const acceptStart=performance.now();await apply.click();await until('exact real Accept',async()=>await text()===edited);sample.acceptMs=performance.now()-acceptStart;sample.actual=await text();sample.status='PASS';
  }catch(e){sample.error=String(e);sample.events=await page.evaluate(()=>window.__analysis);sample.actual=await text();}
  await writeFile('C:/GrammarOutput/progress.json',JSON.stringify(report,null,2));
  if((await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('local_model_status'))).phase!=='ready')break;
 }
''' + s[end:]
s = s.replace(" const cwd=await readFile('C:/PencilTrialScope/install/local/runtime.cwd','utf8');", " const cwd='C:/PencilTrialScope/install/nonexistent-fixture-directory';")
s = s.replace(" report.status='PASS';", " report.status=report.samples.length===5&&report.samples.every(s=>s.status==='PASS')?'PASS':'FAIL';")
s = s.replace("'local-analysis-completed'", "'instant-candidate'")
s = s.replace(" async function capture(prefix='',suffix=''){", " let trialSource='please seperate this.';let captureStart=0;\n async function capture(prefix='',suffix=''){")
s = s.replace("prefix+'please seperate this.'+suffix", "prefix+trialSource+suffix").replace('prefix.length+21','prefix.length+trialSource.length')
s = s.replace("assert.equal(token.selectedText,'please seperate this.')", "assert.equal(token.selectedText,trialSource)")
s = s.replace("assert.equal(await command('HOTKEY'),'SENT 6');", "captureStart=performance.now();assert.equal(await command('HOTKEY'),'SENT 6');")
s = s.replace("for(const [mode,target] of [['natural','en'],['concise','en'],['polite','en'],['translate','ko'],['translate','en']])", "for(const [mode,target,source] of [['natural','en','The meeting was moved to Friday because the team needs more time.'],['concise','en','At this point in time, we are currently reviewing the draft.'],['polite','en','Send me the updated report today.'],['translate','ko','Do not change the price. The total is 25 dollars.'],['translate','en','회의는 금요일에 열립니다. 보고서를 미리 보내 주세요.'],['translate','ko','The team reviewed the report yesterday. We found two errors in the final table. Please send a corrected version by Friday. Do not change the total amount.'],['natural','en','We looked at the report yesterday. There were two mistakes in the last table. Please send us a corrected copy by Friday. The total amount must stay the same.']])")
s = s.replace("const sample={mode,target,source:'please seperate this.',status:'FAIL'}", "trialSource=source;const sample={mode,target,source,status:'FAIL'}")
s = s.replace("const v=await draft.inputValue();return v.length>0&&v!=='please seperate this.';", "const events=await page.evaluate(()=>window.__analysis);if(events.some(e=>e.event==='local-analysis-failed'))throw Error('local analysis refused or timed out');const v=await draft.inputValue();return events.some(e=>e.event==='instant-candidate')&&v.length>0;")
s = s.replace('performance.now()-start;sample.output','performance.now()-captureStart;sample.output')
s = s.replace('report.samples.length===5','report.samples.length===7')
s = s.replace("const acceptStart=performance.now();await apply.click();", "const beforeEvents=await page.evaluate(()=>window.__analysis.length);const acceptStart=performance.now();await apply.click();")
s = s.replace("sample.actual=await text();sample.status='PASS';", "sample.actual=await text();assert.equal(await page.evaluate(()=>window.__analysis.length),beforeEvents);sample.status='PASS';")
(p / 'managed-app-e2e.mjs').write_text(s,encoding='utf-8')
g = (old / 'Guest-App-Writing.ps1').read_text(encoding='utf-8').replace('package-fixture','package-pristine').replace("'--missing-assets','--writing-modes'", "'--real-model'").replace('300000','360000').replace('CURRENT_SOURCE_APP_BROKER_LAUNCHER_SYNTHETIC_ASSET_CATALOG','REAL_GEMMA_INSTALLED_OFFLINE_TRIAL')
if args.local_assets:
    cache=p/'asset-cache'
    cache.mkdir(exist_ok=True)
    for name in ['gemma-4-12b-it-qat-q4_0.gguf','cudart-llama-bin-win-cuda-12.4-x64.zip','llama-b11461-bin-win-cuda-12.4-x64.zip']:
        owned_link(Path('D:/dev/grammar-evidence/grammar-gemma-20261007')/name,cache/name)
    s=s.replace(",'-RuntimeDirectory','C:/GrammarRuntime','-ModelPath','C:/GrammarPayload/model.gguf'",'')
    (p/'managed-app-e2e.mjs').write_text(s,encoding='utf-8')
    g=g.replace(" $arguments=@("," & C:\\PencilTrialScope\\install\\Prepare-GrammarLocal.ps1 -AssetCache C:\\GrammarPayload\\asset-cache | Set-Content C:\\GrammarOutput\\prepare-result.json\n $arguments=@(")
(p / 'Guest-App-Writing.ps1').write_text(g,encoding='utf-8')
xml = f'<Configuration><vGPU>Disable</vGPU><Networking>Disable</Networking><ProtectedClient>Enable</ProtectedClient><ClipboardRedirection>Disable</ClipboardRedirection><AudioInput>Disable</AudioInput><VideoInput>Disable</VideoInput><PrinterRedirection>Disable</PrinterRedirection><MemoryInMB>24576</MemoryInMB><MappedFolders>'
for host,guest,readonly in [(p,'C:\\GrammarPayload',True),(root/'output','C:\\GrammarOutput',False),(Path('D:/dev/grammar-evidence/grammar-gemma-20261007/runtime'),'C:\\GrammarRuntime',True)]:
    xml+=f'<MappedFolder><HostFolder>{host}</HostFolder><SandboxFolder>{guest}</SandboxFolder><ReadOnly>{str(readonly).lower()}</ReadOnly></MappedFolder>'
xml+='</MappedFolders></Configuration>'
(root/'real.wsb').write_text(xml,encoding='utf-8')
print(root)
