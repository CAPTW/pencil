import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");

function cssRule(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = styles.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`, "s"));
  assert.ok(match, `missing CSS rule: ${selector}`);
  return match[1];
}

const rootRule = cssRule(":root");
assert.match(rootRule, /--text-recommended:\s*#[0-9a-f]{6}/i);
assert.match(rootRule, /--text-instruction:\s*#[0-9a-f]{6}/i);
assert.match(rootRule, /--text-guidance:\s*#[0-9a-f]{6}/i);
assert.match(rootRule, /--text-warning:\s*#[0-9a-f]{6}/i);
assert.match(rootRule, /--text-danger:\s*#[0-9a-f]{6}/i);

assert.match(cssRule(".panel"), /overflow-y:\s*auto/);

const cardRule = cssRule(".result-card");
assert.match(cardRule, /flex:\s*1\s+0\s+auto/);
assert.match(cardRule, /min-height:\s*\d+px/);

const resultAreaRule = cssRule(".result-area");
assert.match(resultAreaRule, /flex:\s*1\s+0\s+132px/);
assert.match(resultAreaRule, /min-height:\s*132px/);

const textareaRule = cssRule(".result-area textarea");
assert.match(textareaRule, /color:\s*var\(--text-recommended\)/);
assert.match(textareaRule, /font-size:\s*16px/);
assert.match(textareaRule, /font-weight:\s*500/);

const instructionRule = cssRule(".result-heading > div > span");
assert.match(instructionRule, /color:\s*var\(--text-instruction\)/);
assert.match(instructionRule, /font-size:\s*10px/);

const guidanceRule = cssRule(".rewrite-summary");
assert.match(guidanceRule, /color:\s*var\(--text-guidance\)/);
assert.match(guidanceRule, /font-size:\s*10px/);

const warningRule = cssRule(".terminology-warnings small");
assert.match(warningRule, /color:\s*var\(--text-warning\)/);
assert.match(warningRule, /font-size:\s*9px/);

const errorRule = cssRule(".error-row");
assert.match(errorRule, /flex:\s*0\s+0\s+auto/);
assert.match(errorRule, /color:\s*var\(--text-danger\)/);
assert.match(errorRule, /font-size:\s*10px/);

const actionsRule = cssRule(".result-actions");
assert.match(actionsRule, /flex:\s*0\s+0\s+auto/);
assert.match(actionsRule, /padding-top:\s*10px/);
assert.match(actionsRule, /border-top:\s*1px\s+solid/);

const areaPosition = app.indexOf('<div className="result-area">');
const actionsPosition = app.indexOf('<div className="result-actions">');
assert.ok(areaPosition >= 0 && actionsPosition > areaPosition, "result actions must follow the result box");

console.log("result layout contract passed");
