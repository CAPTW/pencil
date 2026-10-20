import assert from 'node:assert/strict';

// Owned synthetic documents only. The caller owns installation and cleanup.
// Outputs require a separate semantic review; token checks are not quality proof.
export async function runParagraphUI(page, worker, cases, evidence) {
  evidence.rows = [];
  evidence.stale = [];
  evidence.quality = 'PENDING_SEMANTIC_REVIEW';
  const metrics = () => worker.evaluate(() => ({
    requests: gemmaMetrics.requests, responses: gemmaMetrics.responses,
    lastIntent: gemmaMetrics.lastIntent, last: gemmaMetrics.last,
  }));
  const text = el => el.evaluate(e => e.tagName === 'TEXTAREA' ? e.value : e.textContent);
  async function waitIdleResponse(before) {
    await page.waitForFunction(() => !!document.getElementById('grammar-local-assist'));
    const end = Date.now() + 15000;
    while ((await metrics()).responses <= before) {
      assert.ok(Date.now() < end, 'local baseline response deadline');
      await page.waitForTimeout(20);
    }
    await page.waitForTimeout(80);
  }
  async function pause() {
    const button = page.getByRole('button', {name: 'Pause field', exact: true});
    if (await button.count()) await button.click();
  }
  async function prepare(field, source) {
    await pause();
    const el = page.locator('#' + field);
    await el.fill(source);
    await el.click();
    const before = (await metrics()).responses;
    await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
    await waitIdleResponse(before);
    return el;
  }
  async function mode(choice) {
    const select = page.getByLabel('Local review mode', {exact: true});
    await select.focus();
    await select.press('Home');
    for (let n = 0; n < ['paraphrase', 'en', 'ko'].indexOf(choice); n++) await select.press('ArrowDown');
    assert.equal(await select.inputValue(), choice);
  }
  async function review(choice) {
    await mode(choice);
    await page.evaluate(() => {
      window.paragraphClock?.dispose();
      const shadow = document.getElementById('grammar-local-assist').shadowRoot;
      const button = [...shadow.querySelectorAll('button')].find(b => b.textContent === 'Review locally');
      const clock = {};
      let frame;
      const observer = new MutationObserver(() => {
        const suggestion = shadow.querySelector('[aria-label^="Suggestion 1:"]');
        if (!suggestion?.getClientRects().length || clock.start === undefined || clock.dom !== undefined) return;
        clock.dom = performance.now();
        frame = requestAnimationFrame(() => {
          if (suggestion.isConnected && suggestion.getClientRects().length) clock.frame = performance.now();
        });
      });
      const start = () => { clock.start = performance.now(); observer.observe(shadow, {subtree: true, childList: true}); };
      button.addEventListener('click', start, {capture: true, once: true});
      clock.dispose = () => { observer.disconnect(); cancelAnimationFrame(frame); button.removeEventListener('click', start, true); };
      window.paragraphClock = clock;
    });
    await page.getByRole('button', {name: 'Review locally', exact: true}).click();
  }
  async function result(row) {
    await page.waitForFunction(() => Number.isFinite(window.paragraphClock?.frame) || /unavailable|No local change suggested/.test(document.getElementById('grammar-local-assist')?.shadowRoot.querySelector('[role=status]')?.textContent || ''), null, {timeout: 15000});
    assert.ok(await page.evaluate(() => Number.isFinite(window.paragraphClock?.frame)), 'review refused or unchanged; inspect typed response');
    Object.assign(row, await page.evaluate(() => {
      const c = window.paragraphClock;
      c.dispose();
      return {recommendMs: c.frame - c.start, domAvailableMs: c.dom - c.start};
    }));
    const before = (await metrics()).requests;
    const suggestion = page.getByRole('button', {name: /Suggestion 1:/});
    await suggestion.hover();
    await suggestion.click();
    row.output = await page.getByRole('textbox', {name: 'Edit replacement', exact: true}).inputValue();
    assert.equal((await metrics()).requests, before, 'cached card cannot infer');
    row.metrics = await metrics();
  }
  try {
    // Two distinct races: input while translating; source changes inside the
    // final Accept gesture before product mutation. No stale button is retried.
    for (const field of ['writing', 'editable']) {
      const source = cases[0].source;
      const latest = source.replace('3 files', '5 files').replace('120 dollars', '140 dollars');
      const row = {field, kind: 'inflight-repeated-edit', source, latest, status: 'RUNNING'};
      evidence.stale.push(row);
      try {
        const el = await prepare(field, source);
        const base = (await metrics()).requests;
        await review('ko');
        const end = Date.now() + 5000;
        while ((await metrics()).requests <= base) {
          assert.ok(Date.now() < end, 'rewrite admission'); await page.waitForTimeout(10);
        }
        await el.fill(source.replace('3 files', '4 files'));
        await el.fill(latest);
        // Observe beyond normal request deadline, without adding another review.
        await page.waitForTimeout(6000);
        row.sourceAfter = await text(el);
        assert.equal(row.sourceAfter, latest);
        row.cachedLabels = await page.getByRole('button', {name: /Suggestion \d+:/}).allTextContents();
        assert.ok(row.cachedLabels.every(label => { const split = label.indexOf(' → '); return split > 0 && latest.includes(label.slice(0, split)); }), 'every cached source must belong to the latest field, never the old paragraph');
        await review('ko');
        await result(row);
        assert.ok(row.output.includes('5') && row.output.includes('140'), 'latest quantities required');
        const before = (await metrics()).requests;
        await page.getByRole('button', {name: 'Accept', exact: true}).click();
        row.actual = await text(el); assert.equal(row.actual, row.output);
        assert.equal((await metrics()).requests, before);
        row.status = 'PASS_FUNCTIONAL_QUALITY_PENDING';
      } catch (error) { row.status = 'FAIL'; row.error = String(error); row.metrics = await metrics(); }
      finally { await pause(); }

      const race = {field, kind: 'source-change-during-Accept', status: 'RUNNING'};
      evidence.stale.push(race);
      try {
        const el = await prepare(field, source);
        await review('ko'); await result(race);
        await page.evaluate(({field, latest}) => {
          const el = document.getElementById(field);
          const shadow = document.getElementById('grammar-local-assist').shadowRoot;
          const accept = [...shadow.querySelectorAll('button')].find(b => b.textContent === 'Accept');
          accept.addEventListener('mousedown', () => {
            if (el.tagName === 'TEXTAREA') el.value = latest;
            else el.textContent = latest;
          }, {once: true, capture: true});
        }, {field, latest});
        const before = (await metrics()).requests;
        await page.getByRole('button', {name: 'Accept', exact: true}).click({timeout: 3000});
        race.actual = await text(el);
        assert.equal(race.actual, latest, 'stale Accept must preserve exact newer source');
        assert.equal((await metrics()).requests, before, 'stale Accept cannot infer');
        race.status = 'PASS';
      } catch (error) { race.status = 'FAIL'; race.error = String(error); race.metrics = await metrics(); }
      finally { await pause(); }
    }
    for (const sample of cases) for (const field of ['writing', 'editable']) {
      const row = {id: sample.id, field, source: sample.source, qualityCriteria: sample.quality, status: 'RUNNING'};
      evidence.rows.push(row);
      try {
        const el = await prepare(field, sample.source);
        await review(sample.choice);
        await result(row);
        assert.equal(await text(el), sample.source, 'review cannot mutate');
        assert.ok(row.output.trim().length > 0);
        for (const token of sample.required) assert.ok(row.output.includes(token), 'required token: ' + token);
        for (const token of sample.forbidden) assert.ok(!row.output.includes(token), 'forbidden token: ' + token);
        assert.deepEqual(row.metrics.lastIntent, sample.choice === 'paraphrase' ? {mode: 'paraphrase'} : {mode: 'translate', target: sample.choice});
        const start = performance.now();
        await page.getByRole('button', {name: 'Accept', exact: true}).click();
        row.actual = await text(el);
        row.acceptMs = performance.now() - start;
        assert.equal(row.actual, row.output, 'Accept full reread');
        assert.equal((await metrics()).requests, row.metrics.requests, 'Accept cannot infer');
        row.status = 'PASS_FUNCTIONAL_QUALITY_PENDING';
      } catch (error) {
        row.status = 'FAIL'; row.error = String(error);
        row.metrics = await metrics();
        row.uiStatus = await page.locator('[role=status]').allTextContents();
      } finally { await pause(); }
    }
  } finally { await page.evaluate(() => window.paragraphClock?.dispose()); }
  evidence.functionalPass = [...evidence.rows, ...evidence.stale].every(r => r.status.startsWith('PASS'));
  evidence.latencyMisses = evidence.rows.filter(r => r.recommendMs > 3000 || r.acceptMs > 500).map(r => ({id: r.id, field: r.field, recommendMs: r.recommendMs, acceptMs: r.acceptMs}));
  evidence.latencyPass = evidence.functionalPass && evidence.latencyMisses.length === 0;
}
