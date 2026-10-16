use inf1_jup_interface::consts::{
    INF_LST_LIST_ID, INF_MINT_ADDR, RESERVE_V2_LST_LIST_ID, RESERVE_V2_MINT_ADDR, WSOL_MINT_ADDR,
};
use jupiter_amm_interface::{FeeMode, QuoteParams, SwapMode};
use test_utils::{KeyedUiAccount, ALL_FIXTURES};

use crate::common::{swap_test, SwapUserAccs};

fn fixtures_accs(output: &'static str) -> SwapUserAccs<&'static str> {
    SwapUserAccs::default()
        .with_signer("wsol-token-acc-owner")
        .with_inp_token_acc("wsol-token-acc")
        .with_out_token_acc(output)
}

fn add_liq(
    lst_state_list: solana_pubkey::Pubkey,
    amount: u64,
    output_mint: [u8; 32],
    output_account: &'static str,
    swap_mode: SwapMode,
) {
    swap_test(
        lst_state_list,
        QuoteParams {
            amount,
            input_mint: WSOL_MINT_ADDR.into(),
            output_mint: output_mint.into(),
            swap_mode,
            fee_mode: FeeMode::Normal,
        },
        &ALL_FIXTURES,
        fixtures_accs(output_account)
            .map(|name| KeyedUiAccount::from_test_fixtures_json(name).into_keyed_account()),
    );
}

#[test]
fn add_liq_wsol_fixture_basic() {
    add_liq(
        INF_LST_LIST_ID,
        1_000_000_000,
        INF_MINT_ADDR,
        "inf-token-acc",
        SwapMode::ExactIn,
    );
}

#[test]
fn reserve_v2_add_liq_wsol_exact_in_fixture_basic() {
    add_liq(
        RESERVE_V2_LST_LIST_ID,
        1_000,
        RESERVE_V2_MINT_ADDR,
        "reserve-v2-lp-token-acc",
        SwapMode::ExactIn,
    );
}

#[test]
fn reserve_v2_add_liq_wsol_exact_out_fixture_basic() {
    add_liq(
        RESERVE_V2_LST_LIST_ID,
        1_000,
        RESERVE_V2_MINT_ADDR,
        "reserve-v2-lp-token-acc",
        SwapMode::ExactOut,
    );
}
