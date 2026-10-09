// Check a deployment manifest against the chain: every contract must run
// exactly the Wasm the manifest claims and be configured as it claims.
//
// Usage: node scripts/verify-deployment.ts [deployments/testnet.json]
//
// Configuration is read by simulating view calls, which needs any existing
// account as the simulated source: STELLAR_ACCOUNT (an identity or G...
// address) if set, otherwise the policy owner recorded in the manifest.
//
// Needs Node 23.6+ (runs TypeScript directly) and the Stellar CLI on PATH.
// No npm dependencies, so it can run from a bare checkout.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

type Deployment = {
  contractId: string;
  wasmSha256: string;
  gitCommit: string;
  deployedAt: string;
  constructorArgs: Record<string, string>;
};

type Manifest = {
  schemaVersion: number;
  network: "testnet" | "mainnet";
  networkPassphrase: string;
  contracts: Partial<Record<"pactlane-commerce" | "pactlane-evaluation-policy", Deployment>>;
};

const PASSPHRASES: Record<string, string> = {
  testnet: "Test SDF Network ; September 2015",
  mainnet: "Public Global Stellar Network ; September 2015",
};

const manifestPath = process.argv[2] ?? "deployments/testnet.json";
const manifest: Manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
let failures = 0;
const fail = (message: string) => {
  console.error(`FAIL ${message}`);
  failures++;
};
const ok = (message: string) => console.log(`ok   ${message}`);

/** The Stellar CLI's own explanation of a failed call, not just "Command failed". */
const reason = (err: unknown): string => {
  const stderr = String((err as { stderr?: unknown }).stderr ?? "").trim();
  const lines = stderr.split("\n").filter(Boolean);
  return lines.find((line) => /error/i.test(line)) ?? lines.pop() ?? (err as Error).message.split("\n")[0];
};

// Manifest-level checks: a manifest pointing at the wrong network would make
// every other check pass against the wrong chain.
if (manifest.schemaVersion !== 1) fail(`unsupported schemaVersion ${manifest.schemaVersion}`);
if (PASSPHRASES[manifest.network] !== manifest.networkPassphrase) {
  fail(`network "${manifest.network}" does not match passphrase "${manifest.networkPassphrase}"`);
}

const entries = Object.entries(manifest.contracts) as [string, Deployment][];
if (entries.length === 0) {
  console.log(`${manifestPath}: no contracts deployed yet.`);
  process.exit(failures === 0 ? 0 : 1);
}

const stellar = (args: string[]): string =>
  execFileSync("stellar", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();

const source =
  process.env.STELLAR_ACCOUNT ?? manifest.contracts["pactlane-evaluation-policy"]?.constructorArgs.owner;

/** Simulates a no-argument view call and returns its decoded result. */
function view(contractId: string, fn: string): unknown {
  if (!source) throw new Error("set STELLAR_ACCOUNT to an existing account to read configuration");
  const out = stellar([
    "contract", "invoke", "--id", contractId, "--network", manifest.network,
    "--source-account", source, "--send=no", "--", fn,
  ]);
  return JSON.parse(out);
}

/** Compares one on-chain value with what the manifest says. */
function expectView(name: string, contractId: string, fn: string, expected: string, mismatch = fail) {
  try {
    const actual = view(contractId, fn);
    if (actual === expected) ok(`${name} ${fn}() = ${expected}`);
    else mismatch(`${name} ${fn}() is ${actual}, manifest says ${expected}`);
  } catch (err) {
    fail(`${name}: could not read ${fn}(): ${reason(err)}`);
  }
}

const scratch = mkdtempSync(join(tmpdir(), "pactlane-verify-"));
try {
  for (const [name, entry] of entries) {
    // Code: fetch the Wasm the contract instance actually points at.
    const out = join(scratch, `${name}.wasm`);
    try {
      stellar(["contract", "fetch", "--id", entry.contractId, "--network", manifest.network, "--out-file", out]);
      const actual = createHash("sha256").update(readFileSync(out)).digest("hex");
      if (actual === entry.wasmSha256) ok(`${name} ${entry.contractId} runs ${actual}`);
      else fail(`${name}: on-chain Wasm ${actual} != manifest ${entry.wasmSha256}`);
    } catch (err) {
      fail(`${name}: could not fetch ${entry.contractId}: ${reason(err)}`);
    }
  }

  // Configuration: right code with the wrong token or kernel is still wrong.
  const kernel = manifest.contracts["pactlane-commerce"];
  if (kernel) {
    expectView("pactlane-commerce", kernel.contractId, "token", kernel.constructorArgs.token);
  }

  const policy = manifest.contracts["pactlane-evaluation-policy"];
  if (policy) {
    expectView("pactlane-evaluation-policy", policy.contractId, "kernel", policy.constructorArgs.kernel);
    if (kernel && policy.constructorArgs.kernel !== kernel.contractId) {
      fail(`pactlane-evaluation-policy is bound to ${policy.constructorArgs.kernel}, not this manifest's kernel ${kernel.contractId}`);
    }
    // Ownership may legitimately move after deployment; report, don't fail.
    expectView("pactlane-evaluation-policy", policy.contractId, "owner", policy.constructorArgs.owner,
      (message) => console.warn(`warn ${message} (ownership transferred since deployment?)`));
  }
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

process.exit(failures === 0 ? 0 : 1);
