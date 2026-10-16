#![allow(unexpected_cfgs)]
#![cfg(not(target_os = "solana"))]

pub use ::inf1_std;

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
// (`update.rs` holds the `AccountMap` + synthetic Clock glue the live impl mirrors.)

pub use pda::{create_raw_pda, find_pda};
pub use sanctum_lst_list::load_sanctum_lst_list;

pub use consts::{
    INF_LST_LIST_ID, INF_POOL_STATE_ID, RESERVE_V2_LST_LIST_ID, RESERVE_V2_POOL_STATE_ID,
    RESERVE_V2_PROGRAM_ID_PUBKEY as RESERVE_V2_PROGRAM_ID,
};

pub const INF_PROGRAM_ID: solana_pubkey::Pubkey = consts::INF_PROGRAM_ID_PUBKEY;
