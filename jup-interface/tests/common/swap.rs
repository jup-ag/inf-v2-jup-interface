use std::collections::HashMap;

use anyhow::anyhow;
use generic_array_struct::generic_array_struct;
use inf1_jup_interface::InfAmm;
use inf1_std::inf1_ctl_core::{
    instructions::swap::{
        v1::{exact_in::SwapExactInIxData, exact_out::SwapExactOutIxData},
        IxArgs as SwapIxArgs,
    },
    keys::LST_STATE_LIST_ID,
};
use jupiter_amm_interface::{
    Amm, KeyedAccount, QuoteParams, Swap, SwapAndAccountMetas, SwapMode, SwapParams,
};
use mollusk_svm::{
    result::{InstructionResult, ProgramResult},
    Mollusk,
};
use solana_account::Account;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use test_utils::{mollusk_exec, mollusk_inf_fixture_ctl, SPL_LSTS};

use crate::common::AMM_CONTEXT;

thread_local! {
    pub static SVM: Mollusk = mollusk_inf_fixture_ctl()
}

#[generic_array_struct(pub)]
#[derive(Default)]
#[repr(transparent)]
pub struct SwapUserAccs<T> {
    pub signer: T,
    pub inp_token_acc: T,
    pub out_token_acc: T,
}

impl<T> SwapUserAccs<T> {
    pub fn map<R>(self, f: impl FnMut(T) -> R) -> SwapUserAccs<R> {
        SwapUserAccs(self.0.map(f))
    }
}

pub type SwapUserKeyedAccounts = SwapUserAccs<(Pubkey, Account)>;

/// The whole point of it all:
///
/// - inits Amm struct
/// - runs 2x update cycle
/// - quote
/// - swap
/// - mollusk execute swap
/// - assert amount in and out matches quote
pub fn swap_test(
    qp: QuoteParams,
    onchain_state: &HashMap<Pubkey, Account>,
    user: SwapUserKeyedAccounts,
) {
    // init
    let (key, account) = onchain_state
        .get_key_value(&LST_STATE_LIST_ID.into())
        .unwrap();
    let mut inf = InfAmm::new(
        &KeyedAccount {
            key: *key,
            account: account.clone(),
            params: None,
        },
        &AMM_CONTEXT,
        SPL_LSTS.into_iter().collect(),
    )
    .unwrap();

    // 1st update might fail bec it might be based on stale data
    // bec DEFAULT_MAINNET_POOL might be stale
    let _: Result<_, _> = update_cycle(&mut inf, onchain_state);
    // panic if 2nd update cycle fails
    update_cycle_strict(&mut inf, onchain_state).unwrap();

    let quote = inf.quote(&qp).unwrap();
    let saam = inf
        .get_swap_and_account_metas(&SwapParams {
            swap_mode: qp.swap_mode,
            in_amount: quote.in_amount,
            out_amount: quote.out_amount,
            source_mint: qp.input_mint,
            destination_mint: qp.output_mint,
            source_token_account: user.inp_token_acc().0,
            destination_token_account: user.out_token_acc().0,
            token_transfer_authority: user.signer().0,
            // dont-cares
            quote_mint_to_referrer: Default::default(),
            jupiter_program_id: &Default::default(),
            missing_dynamic_accounts_as_default: Default::default(),
        })
        .unwrap();
    let ix = saam_to_inf_ix(qp.amount, saam, qp.swap_mode);

    let (
        accs_bef,
        InstructionResult {
            program_result,
            resulting_accounts,
            ..
        },
    ) = SVM.with(|svm| mollusk_exec(svm, &ix, onchain_state));

    assert!(
        matches!(program_result, ProgramResult::Success),
        "{program_result:#?}"
    );

    assert_balance_change(
        &accs_bef,
        &resulting_accounts,
        &user.inp_token_acc().0,
        quote.in_amount,
        BalanceChangeDir::Dec,
    );
    assert_balance_change(
        &accs_bef,
        &resulting_accounts,
        &user.out_token_acc().0,
        quote.out_amount,
        BalanceChangeDir::Inc,
    );
}

/// Compared to [`update_cycle_strict`], no-ops if an account to update is missing from
/// `onchain_state`
fn update_cycle(inf: &mut InfAmm, onchain_state: &HashMap<Pubkey, Account>) -> anyhow::Result<()> {
    let accs = inf.get_accounts_to_update();
    let am: HashMap<_, _, _> = accs
        .into_iter()
        .filter_map(|pk| {
            // panic if we're missing an account to update
            let (k, v) = onchain_state.get_key_value(&pk)?;
            Some((*k, v.clone()))
        })
        .collect();
    inf.update(&am)
}

fn update_cycle_strict(
    inf: &mut InfAmm,
    onchain_state: &HashMap<Pubkey, Account>,
) -> anyhow::Result<()> {
    let accs = inf.get_accounts_to_update();
    let am: anyhow::Result<HashMap<_, _, _>> = accs
        .into_iter()
        .map(|pk| {
            // panic if we're missing an account to update
            let (k, v) = onchain_state
                .get_key_value(&pk)
                .ok_or_else(|| anyhow!("Missing acc {pk}"))?;
            Ok((*k, v.clone()))
        })
        .collect();
    let account_map = am?;

    inf.update(&account_map)
}

fn saam_to_inf_ix(
    amount: u64,
    SwapAndAccountMetas {
        swap,
        mut account_metas,
    }: SwapAndAccountMetas,
    swap_mode: SwapMode,
) -> Instruction {
    let data = match swap {
        Swap::SanctumS {
            src_lst_value_calc_accs,
            dst_lst_value_calc_accs,
            src_lst_index,
            dst_lst_index,
        } => {
            let ix_args = SwapIxArgs {
                inp_lst_value_calc_accs: src_lst_value_calc_accs,
                out_lst_value_calc_accs: dst_lst_value_calc_accs,
                inp_lst_index: src_lst_index,
                out_lst_index: dst_lst_index,
                amount,
                limit: match swap_mode {
                    SwapMode::ExactIn => 0,
                    SwapMode::ExactOut => u64::MAX,
                },
            };
            match swap_mode {
                SwapMode::ExactIn => SwapExactInIxData::new(&ix_args).as_buf().to_vec(),
                SwapMode::ExactOut => SwapExactOutIxData::new(&ix_args).as_buf().to_vec(),
            }
        }
        _ => unreachable!(),
    };
    // Refer to `get_swap_and_account_metas` to view changes
    // that we made to the vanilla instruction that we need to undo here:
    // - account inserted at front
    // - all is_signer set to false. All swap instructions have
    //   signer as the first account
    account_metas.remove(0);
    account_metas[0].is_signer = true;

    Instruction {
        program_id: inf1_std::inf1_ctl_core::ID.into(),
        accounts: account_metas,
        data,
    }
}

enum BalanceChangeDir {
    Dec,
    Inc,
}

fn assert_balance_change(
    accs_bef: &[(Pubkey, Account)],
    accs_aft: &[(Pubkey, Account)],
    pk: &Pubkey,
    expected_change: u64,
    dir: BalanceChangeDir,
) {
    let [balance_bef, balance_aft] = [accs_bef, accs_aft].map(|arr| {
        let (_pk, acc) = arr.iter().find(|(k, _)| k == pk).unwrap();
        balance_from_token_acc_data(&acc.data).unwrap()
    });
    match dir {
        BalanceChangeDir::Dec => assert_eq!(balance_bef - balance_aft, expected_change),
        BalanceChangeDir::Inc => assert_eq!(balance_aft - balance_bef, expected_change),
    }
}

fn balance_from_token_acc_data(token_acc_data: &[u8]) -> Option<u64> {
    u64_le_at(token_acc_data, 64)
}

fn u64_le_at(data: &[u8], at: usize) -> Option<u64> {
    chunk_at(data, at).map(|c| u64::from_le_bytes(*c))
}

fn chunk_at<const N: usize>(data: &[u8], at: usize) -> Option<&[u8; N]> {
    data.get(at..).and_then(|s| s.first_chunk())
}
