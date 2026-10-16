use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use glob::glob;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use solana_account::Account;
use solana_account_decoder_client_types::UiAccount;
use solana_pubkey::Pubkey;

use crate::{mock_clock, mock_prog_acc, mock_progdata_acc, CONST_PUBKEYS};

pub const FIXTURE_PROGRAMS: [(&str, Pubkey); 9] = [
    ("inf", *CONST_PUBKEYS.inf_ctl_prog()),
    ("lido-calc", *CONST_PUBKEYS.lido_calc_prog()),
    ("marinade-calc", *CONST_PUBKEYS.marinade_calc_prog()),
    (
        "sanctum-spl-multi-calc",
        *CONST_PUBKEYS.sanctum_spl_multi_calc_prog(),
    ),
    ("wsol-calc", *CONST_PUBKEYS.wsol_calc_prog()),
    ("flatslab-pp", *CONST_PUBKEYS.flatslab_pp_prog()),
    ("reserve-v2", *CONST_PUBKEYS.reserve_v2_prog()),
    ("reserve-v2-pp", *CONST_PUBKEYS.reserve_v2_pp_prog()),
    ("inf-svc", *CONST_PUBKEYS.inf_svc_prog()),
];

lazy_static! {
    pub static ref ALL_FIXTURES: HashMap<Pubkey, Account> = {
        let abs_json_paths = glob(test_fixtures_dir().join("*.json").to_str().unwrap()).unwrap();
        abs_json_paths
            .map(|p| KeyedUiAccount::from_file(p.unwrap()).into_keyed_account())
            .chain(
                [
                    (
                        *CONST_PUBKEYS.sanctum_spl_multi_prog(),
                        *CONST_PUBKEYS.sanctum_spl_multi_progdata(),
                    ),
                    (
                        *CONST_PUBKEYS.lido_prog(),
                        *CONST_PUBKEYS.lido_progdata(),
                    ),
                    (
                        *CONST_PUBKEYS.marinade_prog(),
                        *CONST_PUBKEYS.marinade_progdata(),
                    ),
                ]
                .into_iter()
                .flat_map(|(prog_id, prog_data_id)| {
                    [
                        (prog_id, mock_prog_acc(prog_data_id)),
                        (prog_data_id, mock_progdata_acc()),
                    ]
                }),
            )
            .chain(FIXTURE_PROGRAMS.into_iter().map(|(_, prog_id)| {
                (
                    prog_id,
                    // dont-care, doesnt affect mollusk, program is added to ProgramCache
                    // via other mechanism
                    mock_prog_acc(Default::default()),
                )
            }))
            .chain([
                (*CONST_PUBKEYS.sysvar_clock(), mock_clock()),
                mollusk_svm_programs_token::token::keyed_account(),
                mollusk_svm_programs_token::associated_token::keyed_account(),
                mollusk_svm::program::keyed_account_for_system_program(),
            ])
            .collect()
    };
}

/// Continues if fixture account not found for given pubkey
pub fn fixtures_accounts_opt_cloned(
    itr: impl IntoIterator<Item = impl Into<Pubkey>>,
) -> impl Iterator<Item = (Pubkey, Account)> {
    itr.into_iter().filter_map(|pk| {
        let (k, v) = ALL_FIXTURES.get_key_value(&pk.into())?;
        Some((*k, v.clone()))
    })
}

/// Copied from https://stackoverflow.com/a/74942075/5057425
pub fn workspace_root_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

/// Returns `/path/to/workspace/root/test-fixtures`
pub fn test_fixtures_dir() -> PathBuf {
    workspace_root_dir().join("test-fixtures")
}

/// This is the json format of
/// `solana account -o <FILENAME>.json --output json <ACCOUNT-PUBKEY>`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyedUiAccount {
    pub pubkey: String,
    pub account: UiAccount,
}

impl KeyedUiAccount {
    pub fn from_file<P: AsRef<Path>>(json_file_path: P) -> Self {
        let mut file = File::open(json_file_path).unwrap();
        serde_json::from_reader(&mut file).unwrap()
    }

    pub fn from_test_fixtures_json(p: &str) -> Self {
        Self::from_file(test_fixtures_dir().join(p).with_extension("json"))
    }

    pub fn into_keyed_account(self) -> (Pubkey, Account) {
        let Self { pubkey, account } = self;
        (pubkey.parse().unwrap(), account.decode().unwrap())
    }
}
