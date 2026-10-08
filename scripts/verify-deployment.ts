// Check that every contract in a deployment manifest is live on its network and
// runs exactly the Wasm the manifest claims.
//
// Usage: node scripts/verify-deployment.ts [deployments/testnet.json]
//
// Needs Node 23.6+ (runs TypeScript directly) and the Stellar CLI on PATH.
// No npm dependencies, so it can run from a bare checkout.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

type Manifest = {
  network: string;
  networkPassphrase: string;
  contracts: Record<
    string,
    { contractId: string; wasmSha256: string; gitCommit: string; deployedAt: string }
  >;
};

const manifestPath = process.argv[2] ?? "deployments/testnet.json";
const manifest: Manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const entries = Object.entries(manifest.contracts);

if (entries.length === 0) {
  console.log(`${manifestPath}: no contracts deployed yet.`);
  process.exit(0);
}

const scratch = mkdtempSync(join(tmpdir(), "pactlane-verify-"));
let failures = 0;

try {
  for (const [name, entry] of entries) {
    const out = join(scratch, `${name}.wasm`);
    try {
      // Fetches the Wasm the contract instance actually points at on chain.
      execFileSync(
        "stellar",
        ["contract", "fetch", "--id", entry.contractId, "--network", manifest.network, "--out-file", out],
        { stdio: ["ignore", "ignore", "pipe"] },
      );
    } catch (err) {
      console.error(`FAIL ${name}: could not fetch ${entry.contractId}: ${(err as Error).message}`);
      failures++;
      continue;
    }

    const actual = createHash("sha256").update(readFileSync(out)).digest("hex");
    if (actual === entry.wasmSha256) {
      console.log(`ok   ${name} ${entry.contractId} ${actual}`);
    } else {
      console.error(`FAIL ${name}: on-chain Wasm ${actual} != manifest ${entry.wasmSha256}`);
      failures++;
    }
  }
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

process.exit(failures === 0 ? 0 : 1);
