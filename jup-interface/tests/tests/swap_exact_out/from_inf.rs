use inf1_jup_interface::consts::{INF_MINT_ADDR, RESERVE_V2_LST_LIST_ID, WSOL_MINT_ADDR};
use jupiter_amm_interface::{FeeMode, QuoteParams, SwapMode};
use test_utils::{KeyedUiAccount, ALL_FIXTURES};

use crate::common::{swap_test, SwapUserAccs};

#[test]
fn reserve_v2_swap_exact_out_inf_to_wsol_fixture_basic() {
    swap_test(
        RESERVE_V2_LST_LIST_ID,
        QuoteParams {
            amount: 1_000,
            input_mint: INF_MINT_ADDR.into(),
            output_mint: WSOL_MINT_ADDR.into(),
            swap_mode: SwapMode::ExactOut,
            fee_mode: FeeMode::Normal,
        },
        &ALL_FIXTURES,
        SwapUserAccs::default()
            .with_signer("inf-token-acc-owner")
            .with_inp_token_acc("inf-token-acc")
            .with_out_token_acc("wsol-token-acc")
            .map(|name| KeyedUiAccount::from_test_fixtures_json(name).into_keyed_account()),
    );
}
