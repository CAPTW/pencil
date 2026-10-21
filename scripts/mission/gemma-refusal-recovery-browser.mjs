import assert from 'node:assert/strict';

// Owned synthetic UI qualification. Observers never inject model output/errors.
export async function runRefusalRecovery(page, worker, observe, evidence) {
  evidence.attempts = [];
  evidence.status = 'RUNNING';
  const metrics = () => worker.evaluate(() => ({requests: gemmaMetrics.requests,
    responses: gemmaMetrics.responses, last: gemmaMetrics.last}));
  const pause = async () => {
    const button = page.getByRole('button', {name: 'Pause field', exact: true});
    if (await button.count()) await button.click();
  };
  const wait = async before => {
    const end = Date.now() + 15000;
    while ((await metrics()).responses <= before) {
      assert.ok(Date.now() < end, 'actual model response deadline');
      await page.waitForTimeout(25);
    }
    await page.waitForTimeout(100);
  };
  evidence.initialProcesses = await observe.processes();
  assert.ok(evidence.initialProcesses.some(p => p.name === 'llama-server'), 'real model running');
  // Finite predeclared probes. Never retry/replay a refused request automatically.
  const sources = [
    'Would you mind sending the report to Mina when you have a moment?',
    'It is not impossible for Mina to finish the review today.',
    'I will not be able to attend the meeting on Monday.',
    'Could you please let me know whether you would be able to send the report?',
    'There is no reason why we cannot review the report before Friday.',
    'I do not think that Mina should send the report before approval.',
    '지연이 승인하면 민수가 시간이 나면 답신을 보내 주세요.',
    '교수님께서 검토하시면 지연이 확인하면 질의를 보내 주세요.',
  ];
  try {
    for (const source of sources) {
      await pause();
      const el = page.locator('#writing');
      await el.fill(source); await el.click();
      let before = (await metrics()).responses;
      await page.getByRole('button', {name: 'Enable this field', exact: true}).click();
      await wait(before);
      const select = page.getByLabel('Local review mode', {exact: true});
      await select.focus(); await select.press('Home');
      assert.equal(await select.inputValue(), 'paraphrase');
      const logBefore = await observe.refusals();
      const requestsBefore = (await metrics()).requests;
      before = (await metrics()).responses;
      await page.getByRole('button', {name: 'Review locally', exact: true}).click();
      await wait(before);
      const after = await metrics();
      const codes = (await observe.refusals()).slice(logBefore.length);
      const row = {source, response: after.last, codes, processes: await observe.processes()};
      evidence.attempts.push(row);
      assert.equal(await el.inputValue(), source, 'review/refusal cannot mutate');
      assert.deepEqual(row.processes, evidence.initialProcesses, 'model/broker PID and start time must survive');
      if (codes.some(c => /^(semantic_changed|register_changed|honorific_changed|literal_changed|modality_changed)$/.test(c))) {
        assert.ok(after.last.error, 'quality refusal must surface as error');
        assert.deepEqual(after.last.suggestions, [], 'refused output must not publish');
        assert.equal(await page.getByRole('button', {name: /Suggestion 1:/}).count(), 0);
        assert.equal(await page.getByRole('button', {name: 'Accept', exact: true}).count(), 0);
        assert.equal(after.requests, requestsBefore + 1, 'explicit request only; no replay');
        await page.waitForTimeout(500);
        assert.equal((await metrics()).requests, after.requests, 'refusal cannot retry itself');
        evidence.refusal = row;
        evidence.status = 'PASS_ACTUAL_REFUSAL_SAME_PROCESS';
        break;
      }
    }
    assert.ok(evidence.refusal, 'No actual quality refusal observed; not a PASS');
  } catch (error) { evidence.status = 'FAIL'; throw error; }
  finally { await pause(); }
}
