import { readFileSync, writeFileSync } from "node:fs";

const fixturesDir = new URL("../../test-fixtures/", import.meta.url);
// INF mint: 5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm
const infMint = Buffer.from(
  "4757899fb8bedba28778aacd67e568e73470cce90bcd532b6cb618297628824e",
  "hex",
);

normalizePoolState("pool-state");
normalizePoolState("reserve-v2-pool-state");
enableReserveV2InfInput();

function normalizePoolState(name) {
  editAccountData(name, (data) => {
    if (data.length !== 240 || data[12] !== 2) {
      throw new Error(`${name} is not a V2 pool state`);
    }

    data.writeBigUInt64LE(0n, 216); // withheld_lamports
    data.writeBigUInt64LE(0n, 232); // last_release_slot
  });
}

function enableReserveV2InfInput() {
  editAccountData("reserve-v2-lst-state-list", (data) => {
    const lstStateLength = 80;

    for (let offset = 0; offset < data.length; offset += lstStateLength) {
      const mint = data.subarray(offset + 16, offset + 48);
      if (mint.equals(infMint)) {
        data[offset] = 0; // is_input_disabled
        return;
      }
    }

    throw new Error("INF is missing from the Reserve V2 LST state list");
  });
}

function editAccountData(name, edit) {
  const path = new URL(`${name}.json`, fixturesDir);
  const json = readFileSync(path, "utf8");
  const fixture = JSON.parse(json);

  if (fixture.account.data[1] !== "base64") {
    throw new Error(`${name} is not base64 encoded`);
  }

  const encoded = fixture.account.data[0];
  const data = Buffer.from(encoded, "base64");
  edit(data);
  writeFileSync(path, json.replace(encoded, data.toString("base64")));
}
