use inf1_std::{
    inf1_ctl_core::{
        accounts::pool_state::{PoolStateV2, VerPoolState},
        keys::{INF_PROGRAM_ID, RESERVE_V2_PROGRAM_ID},
        pda::{const_find_lst_state_list, const_find_pool_state, CONST_PDA_KEYS_OWNED},
    },
    inf1_pp_ag_std::{inf1_pp_reserve_v2_core, PricingAgTy},
};
use solana_pubkey::Pubkey;

pub const LABEL: &str = "Sanctum Infinity";
pub const RESERVE_V2_LABEL: &str = "Sanctum Reserve V2";

pub const INF_PROGRAM_ID_PUBKEY: Pubkey = Pubkey::new_from_array(INF_PROGRAM_ID);
pub const RESERVE_V2_PROGRAM_ID_PUBKEY: Pubkey = Pubkey::new_from_array(RESERVE_V2_PROGRAM_ID);
pub const INF_LST_LIST_ID: Pubkey = Pubkey::new_from_array(*CONST_PDA_KEYS_OWNED.lst_state_list());
pub const INF_POOL_STATE_ID: Pubkey = Pubkey::new_from_array(*CONST_PDA_KEYS_OWNED.pool_state());
pub const RESERVE_V2_LST_LIST_ID: Pubkey =
    Pubkey::new_from_array(const_find_lst_state_list(&RESERVE_V2_PROGRAM_ID).0);
pub const RESERVE_V2_POOL_STATE_ID: Pubkey =
    Pubkey::new_from_array(const_find_pool_state(&RESERVE_V2_PROGRAM_ID).0);

pub const INF_MINT_ADDR: [u8; 32] =
    Pubkey::from_str_const("5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm").to_bytes();

pub const WSOL_MINT_ADDR: [u8; 32] =
    Pubkey::from_str_const("So11111111111111111111111111111111111111112").to_bytes();

pub const RESERVE_V2_MINT_ADDR: [u8; 32] =
    *inf1_pp_reserve_v2_core::keys::CONST_KEYS_OWNED.lp_mint();

/// Returns bootstrap pool state containing the current mainnet values for
/// fields used by [`jupiter_amm_interface::Amm::get_accounts_to_update`].
/// Live on-chain state replaces these values during the first update.
pub fn initial_pool(program_id: &Pubkey) -> Option<VerPoolState> {
    let (pricing_program, lp_token_mint) = if program_id == &INF_PROGRAM_ID_PUBKEY {
        (*PricingAgTy::FlatFee(()).program_id(), INF_MINT_ADDR)
    } else if program_id == &RESERVE_V2_PROGRAM_ID_PUBKEY {
        (inf1_pp_reserve_v2_core::ID, RESERVE_V2_MINT_ADDR)
    } else {
        return None;
    };

    Some(VerPoolState::V2(PoolStateV2 {
        version: 2,
        pricing_program,
        lp_token_mint,
        ..PoolStateV2::default()
    }))
}
