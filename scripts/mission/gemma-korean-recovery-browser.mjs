import assert from 'node:assert/strict';

// Actual owned-install UI requests only. No output/error injection. A refusal
// proves safe rejection, not the linguistic correctness of the rejected output.
export async function runKoreanRecovery(page, worker, observe, cases, evidence) {
  evidence.rows = [];
  evidence.initialProcesses = await observe.processes();
  const metrics = () => worker.evaluate(() => ({requests: gemmaMetrics.requests,
    responses: gemmaMetrics.responses, last: gemmaMetrics.last}));
  const pause = async () => {
    const b = page.getByRole('button', {name: 'Pause field', exact: true});
    if (await b.count()) await b.click();
  };
  const text = el => el.evaluate(e => e.tagName === 'TEXTAREA' ? e.value : e.textContent);
  async function wait(before) {
    const deadline = Date.now() + 15000;
    while ((await metrics()).responses <= before) {
      assert.ok(Date.now() < deadline, 'actual response deadline');
      await page.waitForTimeout(25);
    }
    await page.waitForTimeout(100);
  }
  async function request(sample, field, recoveryOf = null) {
    const row = {id: sample.id, field, source: sample.source, choice: sample.choice,
      qualityCriteria: sample.quality, recoveryOf, status: 'RUNNING'};
    evidence.rows.push(row);
    try {
      await pause();
      const el = page.locator('#' + field);
      await el.fill(sample.source); await el.click();
      let before = await metrics();
      await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
      await wait(before.responses);
      const select = page.getByLabel('Local review mode', {exact: true});
      await select.focus(); await select.press('Home');
      for (let i = 0; i < ['paraphrase', 'en', 'ko'].indexOf(sample.choice); i++) await select.press('ArrowDown');
      assert.equal(await select.inputValue(), sample.choice);
      const codesBefore = await observe.refusals();
      before = await metrics();
      const start = performance.now();
      await page.getByRole('button', {name: 'Review locally', exact: true}).click();
      await wait(before.responses);
      row.responseObservedMs = performance.now() - start; // includes fixed observation wait; not paint timing
      const after = await metrics();
      row.response = after.last;
      row.codes = (await observe.refusals()).slice(codesBefore.length);
      row.processes = await observe.processes();
      assert.deepEqual(row.processes, evidence.initialProcesses, 'same resident processes');
      assert.equal(await text(el), sample.source, 'review cannot mutate');
      assert.equal(after.requests, before.requests + 1, 'no automatic replay');
      row.uiStatus = await page.locator('[role=status]').allTextContents();
      if (after.last.error) {
        assert.ok(row.codes.some(c => /^(semantic_changed|register_changed|honorific_changed|literal_changed|modality_changed)$/.test(c)), 'error is an observed quality refusal');
        assert.deepEqual(after.last.suggestions, []);
        assert.equal(await page.getByRole('button', {name: /Suggestion 1:/}).count(), 0);
        assert.equal(await page.getByRole('button', {name: 'Accept', exact: true}).count(), 0);
        await page.waitForTimeout(500);
        assert.equal((await metrics()).requests, after.requests, 'refusal cannot replay');
        assert.equal(await text(el), sample.source);
        row.status = 'PASS_REFUSAL_BOUNDARY';
      } else if (!after.last.suggestions.length) {
        row.status = 'NO_CHANGE_NOT_ACCEPT_QUALIFICATION';
      } else {
        const suggestion = page.getByRole('button', {name: /Suggestion 1:/});
        await suggestion.hover(); await suggestion.click();
        row.output = await page.getByRole('textbox', {name: 'Edit replacement', exact: true}).inputValue();
        assert.ok(row.output.trim());
        for (const token of sample.required ?? []) assert.ok(row.output.includes(token), 'required literal ' + token);
        for (const pattern of sample.requiredPatterns ?? []) assert.match(row.output, new RegExp(pattern, 'i'));
        for (const pattern of sample.forbiddenPatterns ?? []) assert.doesNotMatch(row.output, new RegExp(pattern, 'i'));
        for (const token of sample.forbidden ?? []) assert.ok(!row.output.includes(token), 'forbidden wording ' + token);
        const acceptStart = performance.now();
        await page.getByRole('button', {name: 'Accept', exact: true}).click();
        row.actual = await text(el); row.acceptMs = performance.now() - acceptStart;
        assert.equal(row.actual, row.output, 'exact Accept reread');
        assert.equal((await metrics()).requests, after.requests, 'cache and Accept cannot infer');
        row.status = 'PASS_FUNCTIONAL_QUALITY_PENDING';
      }
      if (recoveryOf) assert.equal(row.status, 'PASS_FUNCTIONAL_QUALITY_PENDING', 'recovery must produce and Accept a real suggestion');
    } catch (e) { row.status = 'FAIL'; row.error = String(e); }
    finally { await pause(); }
    return row;
  }
  const recovery = cases.filter(c => c.recovery);
  for (const sample of cases) for (const field of ['writing', 'editable']) {
    const row = await request(sample, field);
    if (row.status === 'PASS_REFUSAL_BOUNDARY') {
      for (const next of recovery) await request(next, field, sample.id);
    }
  }
  evidence.afterFollowupProcesses = await observe.processes();
  assert.deepEqual(evidence.afterFollowupProcesses, evidence.initialProcesses);
  evidence.refusalTypes = [...new Set(evidence.rows.flatMap(r => r.codes ?? []))];
  evidence.functionalPass = evidence.rows.every(r => r.status !== 'FAIL');
  evidence.multipleRefusalTypesObserved = evidence.refusalTypes.length >= 2;
  evidence.quality = 'PENDING_MANUAL_SEMANTIC_REVIEW';
  evidence.status = evidence.functionalPass ? 'PASS_BOUNDARIES_QUALITY_PENDING' : 'FAIL';
}
