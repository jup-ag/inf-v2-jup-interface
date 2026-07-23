#![allow(unexpected_cfgs)]
#![cfg(not(target_os = "solana"))]

pub use ::inf1_std;
use inf1_std::{inf1_ctl_core::keys::CONST_KEYS_OWNED, pda::CONST_PDA_KEYS_OWNED};
use solana_pubkey::Pubkey;

pub mod clock;
pub mod consts;
pub mod err;
pub mod pda;
pub mod sanctum_lst_list;

// Reference-only, NOT compiled: the live Jupiter `Amm` impl and `AccountMap`
// glue are owned by jupiter-core (`src/amms/inf_v2_amm.rs`) so this crate
// stays free of any published `jupiter-amm-interface` version.
// pub mod amm;
// pub mod update;

pub use pda::{create_raw_pda, find_pda};
pub use sanctum_lst_list::load_sanctum_lst_list;

pub const INF_PROGRAM_ID: Pubkey = Pubkey::new_from_array(*CONST_KEYS_OWNED.program());
pub const INF_LST_LIST_ID: Pubkey = Pubkey::new_from_array(*CONST_PDA_KEYS_OWNED.lst_state_list());
