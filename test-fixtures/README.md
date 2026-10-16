# test-fixtures

## Clone Program

```sh
solana program dump <addr> <programs/out.so>
```

Then, add `--upgradeable-program <addr> <programs/out.so> none` to the docker compose file to add it to the local validator.

## Clone Account

```sh
solana account --output json -o <account.json> <addr>
```

## Modify Accounts

- Set `withheld_lamports=0` and `last_release_slot=0` on INF and Reserve V2 pool state fixtures.
- Set `is_input_disabled=0` for INF on the Reserve V2 LST state list fixture.

```sh
node test-utils/scripts/prepare-reserve-v2-test-fixtures.mjs
```

## Create Accounts

- Create the Reserve V2 LP token account fixture.

```sh
node test-utils/scripts/create-reserve-v2-lp-token-account.mjs
```
