import assert from 'node:assert/strict';
import {test} from 'node:test';
import {requestAppQuit} from './request-app-quit.mjs';
test('normal IPC reply is not process exit proof', async () => {
  assert.deepEqual(await requestAppQuit({evaluate: async () => {}}), {reply:'received'});
});
test('closed transport is recorded, not marked process success', async () => {
  const result=await requestAppQuit({evaluate: async () => {
    throw Error('page.evaluate: Target page, context or browser has been closed');
  }});
  assert.equal(result.reply,'transport_closed_before_reply');
  assert.equal(result.appNatural,undefined);
});
test('unrelated quit failure is still an error', async () => {
  await assert.rejects(requestAppQuit({evaluate: async () => {throw Error('IPC permission denied');}}),/permission denied/);
});
