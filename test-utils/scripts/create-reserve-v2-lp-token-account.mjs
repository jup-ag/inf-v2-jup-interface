import { readFileSync, writeFileSync } from "node:fs";

const fixturesDir = new URL("../../test-fixtures/", import.meta.url);
const sourcePath = new URL("inf-token-acc.json", fixturesDir);
const outputPath = new URL("reserve-v2-lp-token-acc.json", fixturesDir);

const json = readFileSync(sourcePath, "utf8");
const fixture = JSON.parse(json);
// random pubkey
const tokenAccountPubkey = "EDg5Mfuj1XjaRNCunJe6KcqKi6Fg6P33zLZhkBSjfoun";

if (fixture.account.data[1] !== "base64") {
  throw new Error("inf-token-acc is not base64 encoded");
}

const encoded = fixture.account.data[0];
const data = Buffer.from(encoded, "base64");
if (data.length !== 165) {
  throw new Error("inf-token-acc is not an SPL token account");
}

// Reserve V2 LP mint: SRV2G6xv3ExJtRJHEPqbWCPg2baNU2Z5JBhb2KqqDee
const reserveV2Mint = Buffer.from(
  "06833411a0238dfa856836fceb1a187a55b071a0534998b7cbac0145f36ce137",
  "hex",
);
reserveV2Mint.copy(data, 0);
data.writeBigUInt64LE(1_000_000n, 64);

const output = json
  .replace(fixture.pubkey, tokenAccountPubkey)
  .replace(encoded, data.toString("base64"));
writeFileSync(outputPath, output);
