use inf1_jup_interface::consts::INF_MINT_ADDR;
use jupiter_amm_interface::{FeeMode, QuoteParams, SwapMode};
use test_utils::{KeyedUiAccount, ALL_FIXTURES, CONST_PUBKEYS};

use crate::common::{swap_test, SwapUserAccs};

fn fixtures_accs() -> SwapUserAccs<&'static str> {
    SwapUserAccs::default()
        .with_signer("inf-token-acc-owner")
        .with_inp_token_acc("inf-token-acc")
        .with_out_token_acc("jupsol-token-acc")
}

#[test]
fn remove_liq_spl_fixture_basic() {
    swap_test(
        QuoteParams {
            amount: 1_000_000_000,
            input_mint: INF_MINT_ADDR.into(),
            output_mint: *CONST_PUBKEYS.jupsol_mint(),
            swap_mode: SwapMode::ExactIn,
            fee_mode: FeeMode::Normal,
        },
        &ALL_FIXTURES,
        fixtures_accs().map(|n| KeyedUiAccount::from_test_fixtures_json(n).into_keyed_account()),
    );
}
