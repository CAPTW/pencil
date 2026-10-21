import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';

// Actual installed host protocol, owned synthetic text only. No UI permission claim.
export async function probeActualRefusal(hostPath, origin, observe, evidence) {
  assert.equal(process.env.USERNAME, 'WDAGUtilityAccount');
  evidence.status = 'RUNNING'; evidence.attempts = [];
  evidence.initialProcesses = await observe.processes();
  for (const name of ['llama-server', 'grammar-gemma-broker'])
    assert.ok(evidence.initialProcesses.some(p => p.name === name));
  const child = spawn(hostPath, [origin], {windowsHide: true, stdio: ['pipe','pipe','pipe']});
  let buffered = Buffer.alloc(0), pending, fault;
  const fail = error => { fault = error; pending?.reject(error); pending = null; };
  child.on('error', fail);
  child.on('exit', code => { if (pending) fail(Error('native host exited: ' + code)); });
  child.stderr.resume();
  child.stdout.on('data', data => {
    buffered = Buffer.concat([buffered, data]);
    if (buffered.length > 256 * 1024 + 4) return fail(Error('response bound'));
    if (buffered.length < 4) return;
    const size = buffered.readUInt32LE();
    if (size > 256 * 1024) return fail(Error('frame bound'));
    if (buffered.length < size + 4) return;
    try {
      const reply = JSON.parse(buffered.subarray(4, size + 4));
      buffered = buffered.subarray(size + 4);
      assert.ok(pending, 'unsolicited response');
      const resolve = pending.resolve; pending = null; resolve(reply);
    } catch (error) { fail(error); }
  });
  async function request(text, index) {
    if (fault) throw fault;
    const body = Buffer.from(JSON.stringify({version:1, op:'analyze', id:String(index),
      epoch:'owned-refusal-probe', revision:index, text, intent:{mode:'paraphrase'}}));
    const header = Buffer.alloc(4); header.writeUInt32LE(body.length);
    let timer;
    try { return await new Promise((resolve, reject) => {
      pending = {resolve, reject}; timer = setTimeout(() => fail(Error('response deadline')), 15000);
      child.stdin.write(Buffer.concat([header, body]));
    }); } finally { clearTimeout(timer); }
  }
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
    for (const [index, source] of sources.entries()) {
      const before = await observe.refusals();
      const response = await request(source, index + 1);
      await new Promise(r => setTimeout(r, 100));
      const codes = (await observe.refusals()).slice(before.length);
      const row = {source, response, codes, processes:await observe.processes()};
      evidence.attempts.push(row);
      assert.deepEqual(row.processes, evidence.initialProcesses, 'same model and broker identity');
      if (codes.some(c => /^(semantic_changed|register_changed|honorific_changed|literal_changed|modality_changed)$/.test(c))) {
        assert.ok(response.error); assert.deepEqual(response.suggestions, []);
        evidence.refusal = row; evidence.status = 'PASS_ACTUAL_REFUSAL_SAME_PROCESS'; break;
      }
    }
    assert.ok(evidence.refusal, 'No actual quality refusal; not qualified');
  } catch (error) { evidence.status = 'FAIL'; throw error; }
  finally {
    child.stdin.end();
    const end = Date.now() + 5000;
    while (child.exitCode === null && Date.now() < end) await new Promise(r => setTimeout(r, 25));
    evidence.hostExit = child.exitCode;
    if (child.exitCode === null) { child.kill(); evidence.forcedHostCleanup = true; }
    assert.equal(evidence.hostExit, 0, 'native host must exit naturally');
  }
}
