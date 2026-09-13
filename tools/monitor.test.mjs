import {test} from 'node:test';
import assert from 'node:assert/strict';
import {keysFor, summarize, monitor} from './monitor.mjs';
import {xdr} from '@stellar/stellar-sdk';
const registry = {schema:'ledgerparity-contract-registry/v1', rpc_url:'https://soroban-testnet.stellar.org', network:'Test SDF Network ; September 2015',
  contract_id:'CBSEWOCE3V7SZDYKAOIJDXI54KWPFIHU7RMRTB56PZBV5Y3N47BGNPV7', wasm_sha256:'26c92f94678817fb97dc8e4f9e7d9858d4f39d28c1bae20420201804d1a483cd', reports:['a'.repeat(64)], renewal_threshold:259200};
test('report restoration key is raw bytes, with separate instance and code keys', () => {
  const keys = keysFor(registry);
  assert.equal(new Set(keys.map(k=>k.key)).size,3);
  const key = xdr.ScVal.fromXDR(keys[2].restore_key_xdr, 'base64');
  assert.equal(key.type, 'scvBytes');
  assert.equal(Buffer.from(key.bytes.value).toString('hex'), registry.reports[0]);
});
test('missing, expired and threshold entries remain maintenance findings', () => {
  const keys=keysFor(registry);
  const result=summarize(keys,{latestLedger:100,entries:[{key:keys[0].key,liveUntilLedgerSeq:1000},{key:keys[1].key,liveUntilLedgerSeq:110}]},10);
  assert.deepEqual(result.entries.map(r=>r.status),['live','renewal_due','missing_or_archived']);
  assert.equal(result.maintenance_required,true);
  assert.equal(summarize(keys,{latestLedger:100,entries:keys.map(k=>({key:k.key,liveUntilLedgerSeq:200}))},10).maintenance_required,false);
  assert.equal(summarize(keys,{latestLedger:100,entries:[{key:keys[0].key,liveUntilLedgerSeq:99}]},10).entries[0].status,'missing_or_archived');
});
test('bad registries and malformed provider entries fail', () => {
  for (const patch of [{reports:['bad']},{reports:['a'.repeat(64),'a'.repeat(64)]},{rpc_url:'http://example.com'},{renewal_threshold:-1}]) assert.throws(()=>keysFor({...registry,...patch}));
  assert.throws(()=>summarize(keysFor(registry),{latestLedger:1,entries:[{key:'unexpected',liveUntilLedgerSeq:10}]},1));
});
test('wrong-network and HTTP errors cannot become healthy results', async () => {
  await assert.rejects(monitor(registry,async()=>({ok:true,json:async()=>({result:{passphrase:'wrong'}})})),/network/);
  await assert.rejects(monitor(registry,async()=>({ok:false,status:503})),/503/);
});
