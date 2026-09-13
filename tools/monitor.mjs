import fs from 'node:fs';
import {pathToFileURL} from 'node:url';
import {Address, xdr} from '@stellar/stellar-sdk';

export function keysFor(registry) {
  if (registry?.schema !== 'ledgerparity-contract-registry/v1' || typeof registry.network !== 'string' ||
      !Array.isArray(registry.reports) || registry.reports.length > 98 || !Number.isSafeInteger(registry.renewal_threshold) || registry.renewal_threshold < 0) {
    throw new Error('Invalid registry (at most 98 reports per file).');
  }
  const url = new URL(registry.rpc_url);
  if (url.protocol !== 'https:' || url.username || url.password) throw new Error('Use an HTTPS RPC URL without embedded credentials.');
  if (!/^[a-f0-9]{64}$/.test(registry.wasm_sha256)) throw new Error('Invalid Wasm hash.');
  if (new Set(registry.reports).size !== registry.reports.length || registry.reports.some(hash => !/^[a-f0-9]{64}$/.test(hash))) throw new Error('Invalid or duplicate report hash.');
  if (!registry.contract_id?.startsWith('C')) throw new Error('Expected a contract address.');
  const contract = new Address(registry.contract_id).toScAddress();
  const dataKey = key => xdr.LedgerKey.contractData(new xdr.LedgerKeyContractData({contract, key, durability:xdr.ContractDataDurability.persistent})).toXDR('base64');
  return [
    {kind:'instance', key:dataKey(xdr.ScVal.scvLedgerKeyContractInstance())},
    {kind:'code', key:xdr.LedgerKey.contractCode(new xdr.LedgerKeyContractCode({hash:Buffer.from(registry.wasm_sha256, 'hex')})).toXDR('base64')},
    ...registry.reports.map(hash => ({kind:'report', hash, key:dataKey(xdr.ScVal.scvBytes(Buffer.from(hash,'hex'))), restore_key_xdr:xdr.ScVal.scvBytes(Buffer.from(hash,'hex')).toXDR('base64')}))
  ];
}

export function summarize(keys, result, threshold) {
  if (!Number.isSafeInteger(result?.latestLedger) || result.latestLedger < 0 || !Array.isArray(result.entries)) throw new Error('Invalid ledger response.');
  const entries = new Map();
  for (const entry of result.entries) {
    if (!keys.some(k => k.key === entry.key) || entries.has(entry.key) || !Number.isSafeInteger(entry.liveUntilLedgerSeq)) throw new Error('Invalid or unexpected ledger entry.');
    entries.set(entry.key, entry);
  }
  const rows = keys.map(item => {
    const entry = entries.get(item.key);
    if (!entry) return {...item, status:'missing_or_archived'};
    const ttl = entry.liveUntilLedgerSeq - result.latestLedger;
    return {...item, live_until_ledger:entry.liveUntilLedgerSeq, ttl_ledgers:ttl, status:ttl < 0 ? 'missing_or_archived' : ttl <= threshold ? 'renewal_due' : 'live'};
  });
  return {schema:'ledgerparity-contract-monitor/v1', latest_ledger:result.latestLedger, entries:rows, maintenance_required:rows.some(row => row.status !== 'live')};
}

export async function monitor(registry, request = fetch) {
  const keys = keysFor(registry);
  const rpc = async (method, params) => {
    const response = await request(registry.rpc_url, {method:'POST', headers:{'content-type':'application/json'},
      body:JSON.stringify({jsonrpc:'2.0', id:1, method, params}), signal:AbortSignal.timeout(30000)});
    if (!response.ok) throw new Error('RPC HTTP ' + response.status);
    const data = await response.json();
    if (data.error || !data.result) throw new Error('RPC failed for ' + method);
    return data.result;
  };
  const network = await rpc('getNetwork', {});
  if (network.passphrase !== registry.network) throw new Error('RPC network differs from registry.');
  const result = summarize(keys, await rpc('getLedgerEntries', {keys:keys.map(k => k.key)}), registry.renewal_threshold);
  return {...result, contract_id:registry.contract_id, network:network.passphrase, protocol_version:network.protocolVersion, observed_at:new Date().toISOString()};
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (process.argv.length !== 3) throw new Error('Usage: node monitor.mjs registry.json');
    if (fs.statSync(process.argv[2]).size > 1024*1024) throw new Error('Registry exceeds 1 MiB.');
    const result = await monitor(JSON.parse(fs.readFileSync(process.argv[2], 'utf8').replace(/^\uFEFF/, '')));
    process.stdout.write(JSON.stringify(result, null, 2) + '\n');
    process.exitCode = result.maintenance_required ? 3 : 0;
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
