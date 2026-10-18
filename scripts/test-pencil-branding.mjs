import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const read=p=>readFileSync(new URL('../'+p,import.meta.url),'utf8');
const config=JSON.parse(read('src-tauri/tauri.conf.json'));
assert.equal(config.productName,'Pencil');
assert.ok(read('index.html').includes('<title>Pencil</title>'));
assert.equal(config.app.windows[0].title,'Pencil');
assert.equal(JSON.parse(read('adapters/chromium/extension/manifest.json')).name,'Pencil');
assert.ok(read('src/App.tsx').includes('<h1>Pencil</h1>'));
assert.ok(read('src-tauri/src/main.rs').includes('.tooltip("Pencil")'));
// Product naming is not a migration of settings/native messaging or persisted mode IDs.
assert.ok(read('src/App.tsx').includes('{ id: "grammar", label: "Grammar"'));
assert.ok(read('scripts/package/Install-Grammar.ps1').includes('GrammarPersonal'));
console.log('Pencil visible naming and compatibility contract PASS');
