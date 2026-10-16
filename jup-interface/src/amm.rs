//! Reference-only: NOT compiled (no `mod` decl in lib.rs).
//! The live Jupiter `Amm` impl is owned by jupiter-core (`src/amms/inf_v2_amm.rs`)
//! so this crate stays free of any published `jupiter-amm-interface` version.
//!
//! Mirrors upstream `igneous-labs/inf-jup-interface` `jup-interface/src/lib.rs`.

use std::{collections::HashMap, iter::once, sync::atomic::Ordering};

use ::sanctum_lst_list::{PoolInfo, SanctumLst};
use anyhow::{anyhow, Context, Result};
use inf1_std::{
    err::InfErr,
    inf1_ctl_core::{accounts::lst_state_list::LstStatePackedList, typedefs::lst_state::LstState},
    inf1_pp_ag_std::{
        inf1_pp_flatfee_core, inf1_pp_flatslab_core, inf1_pp_reserve_v2_core,
        update::{all::AccountsToUpdateAll, UpdatePricingProg},
    },
    inf1_pp_core::pair::Pair,
    inf1_svc_ag_std::{
        inf1_svc_inf_std::SYSVAR_CLOCK, inf1_svc_lido_core, inf1_svc_marinade_core,
        inf1_svc_spl_core, inf1_svc_wsol_core, update::UpdateSvc,
    },
    instructions::swap::v2::{
        exact_in::{swap_exact_in_v2_ix_is_writer, swap_exact_in_v2_ix_keys_owned},
        exact_out::{swap_exact_out_v2_ix_is_writer, swap_exact_out_v2_ix_keys_owned},
    },
    trade::{instruction::TradeIxArgs, Trade, TradeLimitTy},
    update::UpdateErr,
    InfStd,
};
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, AmmLabel, AmmProgramIdToLabel, ClockRef, KeyedAccount, Quote,
    QuoteParams, Swap, SwapAndAccountMetas, SwapMode, SwapParams,
};
use rust_decimal::Decimal;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::{
    consts::{
        initial_pool, INF_PROGRAM_ID_PUBKEY, LABEL, RESERVE_V2_LABEL, RESERVE_V2_PROGRAM_ID_PUBKEY,
    },
    err::FmtErr,
    pda::{create_raw_pda, find_pda},
    sanctum_lst_list::load_sanctum_lst_list,
    update::{sim_clock, AccountMapRef},
};



pub const INF_PROGRAM_ID: Pubkey = INF_PROGRAM_ID_PUBKEY;

// Jupiter supplies Clock through `AmmContext`, not the account map.
// `update()` exposes it to the calculators as a synthetic sysvar account.

fn build_spl_lsts() -> HashMap<[u8; 32], [u8; 32]> {
    load_sanctum_lst_list()
        .into_iter()
        .filter_map(|SanctumLst { mint, pool, .. }| {
            let stake_pool_address = match pool {
                PoolInfo::Lido => return None,
                PoolInfo::Marinade => return None,
                PoolInfo::ReservePool => return None,
                PoolInfo::SanctumSpl(spl_pool_accounts) => spl_pool_accounts.pool.to_bytes(),
                PoolInfo::Spl(spl_pool_accounts) => spl_pool_accounts.pool.to_bytes(),
                PoolInfo::SPool(_) => return None,
                PoolInfo::SanctumSplMulti(spl_pool_accounts) => spl_pool_accounts.pool.to_bytes(),
            };
            Some((mint.to_bytes(), stake_pool_address))
        })
        .collect()
}

#[derive(Clone)]
pub struct InfAmm {
    pub inner: InfStd,
    pub clock_ref: ClockRef,
}

impl AmmProgramIdToLabel for InfAmm {
    const PROGRAM_ID_TO_LABELS: &[(Pubkey, AmmLabel)] = &[
        (INF_PROGRAM_ID_PUBKEY, LABEL),
        (RESERVE_V2_PROGRAM_ID_PUBKEY, RESERVE_V2_LABEL),
    ];
}

impl InfAmm {
    fn pool_state_id(&self) -> [u8; 32] {
        self.inner
            .find_pool_state()
            .expect("pool state PDA cached in InfAmm::new")
            .0
    }

    fn lst_state_list_id(&self) -> [u8; 32] {
        self.inner
            .find_lst_state_list()
            .expect("LST state list PDA cached in InfAmm::new")
            .0
    }

    pub fn new(
        keyed_account: &KeyedAccount,
        amm_context: &AmmContext,
        spl_lsts: HashMap<[u8; 32], [u8; 32]>,
    ) -> Result<Self> {
        let program_id = keyed_account.account.owner;
        let initial_pool = initial_pool(&program_id)
            .ok_or_else(|| anyhow!("Unsupported controller program: {program_id}"))?;
        let mut inner = InfStd::new(
            Some(program_id.to_bytes()),
            initial_pool,
            keyed_account.account.data.clone().into_boxed_slice(),
            None,
            None,
            Default::default(),
            Default::default(),
            spl_lsts,
            find_pda,
            create_raw_pda,
        )
        .map_err(FmtErr)?;
        let lst_state_list_id = inner
            .find_cache_lst_state_list()
            .ok_or(FmtErr(InfErr::NoValidPda))?
            .0;
        inner
            .find_cache_pool_state()
            .ok_or(FmtErr(InfErr::NoValidPda))?;
        if keyed_account.key.to_bytes() != lst_state_list_id {
            return Err(anyhow!("Incorrect LST state list keyed_account"));
        }

        let mut res = Self {
            inner,
            clock_ref: amm_context.clock_ref.clone(),
        };

        // need to initialize sol val calc data for all LSTs on the list
        // so that first update doesnt fail with InfErr::MissingSvcData

        let lst_state_list = LstStatePackedList::of_acc_data(&keyed_account.account.data)
            .context("LstStatePackedList::of_acc_data failed")?;
        lst_state_list
            .0
            .iter()
            .try_for_each(
                |s| match res.inner.try_get_or_init_lst_svc(&s.into_lst_state()) {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        // Do not cause an error when we don't have the necessary spl data for a LST
                        if matches!(error, InfErr::MissingSplData { .. }) {
                            Ok(())
                        } else {
                            Err(error)
                        }
                    }
                },
            )
            .map_err(FmtErr)?;

        Ok(res)
    }
}

impl Amm for InfAmm {
    /// The `keyed_account` should be the `LST_STATE_LIST`, **NOT** `POOL_STATE`.
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> Result<Self>
    where
        Self: Sized,
    {
        Self::new(keyed_account, amm_context, build_spl_lsts())
    }

    fn label(&self) -> String {
        Self::PROGRAM_ID_TO_LABELS
            .iter()
            .find(|(program_id, _)| *program_id == self.program_id())
            .map(|(_, label)| (*label).to_owned())
            .expect("program ID missing from InfAmm::PROGRAM_ID_TO_LABELS")
    }

    fn program_id(&self) -> Pubkey {
        Pubkey::new_from_array(*self.inner.prog_id())
    }

    /// Each controller program has one pool, identified by its LST state list account ID
    fn key(&self) -> Pubkey {
        Pubkey::new_from_array(self.lst_state_list_id())
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        let lst_state_list = self.inner.try_lst_state_list().unwrap_or_default();
        lst_state_list
            .iter()
            .map(|s| s.into_lst_state().mint.into())
            .chain(once((*self.inner.pool.lp_token_mint()).into()))
            .collect()
    }

    /// Note: does not dedup
    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let lst_state_iter = self
            .inner
            .try_lst_state_list()
            .unwrap_or_default() // TODO: should this panic instead if LstStateList format unexpectedly changed?
            .iter()
            .map(|l| l.into_lst_state());
        [
            self.pool_state_id(),
            self.lst_state_list_id(),
            *self.inner.pool.lp_token_mint(),
        ]
        .into_iter()
        .chain(
            self.inner
                .pricing
                .accounts_to_update_all(lst_state_iter.clone().map(|LstState { mint, .. }| mint)),
        )
        .chain(
            lst_state_iter
                .filter_map(|lst_state| {
                    // ignore err here, some LSTs may not have their.
                    // sol val calc accounts fetched yet.
                    //
                    // update() should call `try_get_or_init_lst_svc_mut`
                    // which will make it no longer err for the next update cycle
                    self.inner.accounts_to_update_lst(&lst_state).ok()
                })
                .flatten(),
        )
        .filter(|pk| *pk != SYSVAR_CLOCK)
        .map(Pubkey::new_from_array)
        .collect()
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let pool_state_id = self.pool_state_id();
        let lst_state_list_id = self.lst_state_list_id();
        let clock = sim_clock(&self.clock_ref);
        let fetched = AccountMapRef {
            account_map,
            clock: &clock,
        };
        self.inner.update_pool(fetched).map_err(FmtErr)?;
        self.inner.update_lst_state_list(fetched).map_err(FmtErr)?;
        self.inner.update_lp_token_supply(fetched).map_err(FmtErr)?;

        let InfStd {
            lst_state_list_data,
            pricing,
            lst_calcs,
            spl_lsts,
            lst_reserves,
            create_pda,
            ..
        } = &mut self.inner;

        let mut all_lst_states = LstStatePackedList::of_acc_data(lst_state_list_data)
            .ok_or(FmtErr(InfErr::AccDeser {
                pk: lst_state_list_id,
            }))?
            .0
            .iter()
            .map(|s| s.into_lst_state());

        pricing
            .update_all(
                all_lst_states.clone().map(|LstState { mint, .. }| mint),
                fetched,
            )
            .map_err(|e| e.map_inner(InfErr::UpdatePp))
            .map_err(FmtErr)?;

        all_lst_states
            .try_for_each(|lst_state| {
                InfStd::update_lst_reserves(
                    lst_reserves,
                    create_pda as &_,
                    &pool_state_id,
                    &lst_state,
                    fetched,
                )?;

                let calc =
                    match InfStd::try_get_or_init_lst_svc_static(lst_calcs, spl_lsts, &lst_state) {
                        Ok(calc) => calc,
                        Err(error) => {
                            // Do not cause an error when we don't have the necessary spl data for a LST
                            if matches!(error, InfErr::MissingSplData { .. }) {
                                lst_calcs.remove(&lst_state.mint);
                                return Ok(());
                            } else {
                                return Err(UpdateErr::Inner(error));
                            }
                        }
                    };

                calc.update_svc(fetched)
                    .map_err(|e| e.map_inner(InfErr::UpdateSvc))
            })
            .map_err(FmtErr)?;

        Ok(())
    }

    fn quote(
        &self,
        QuoteParams {
            amount,
            input_mint,
            output_mint,
            swap_mode,
            ..
        }: &QuoteParams,
    ) -> Result<Quote> {
        let current_slot = self.clock_ref.slot.load(Ordering::Relaxed);
        let last_release_slot = self.inner.pool.migrated(0).last_release_slot;
        let slot_lookahead = current_slot.saturating_sub(last_release_slot);
        let quote = self
            .inner
            .quote_trade(
                &Pair {
                    inp: input_mint.as_array(),
                    out: output_mint.as_array(),
                },
                *amount,
                slot_lookahead,
                swap_mode_to_trade_limit_ty(*swap_mode),
            )
            .map_err(FmtErr)?;

        Ok(Quote {
            in_amount: quote.inp,
            out_amount: quote.out,
            // INF fees are denominated in SOL value, which may be neither swap mint
            // Jupiter no longer needs fee reporting in the quote response so just set to 0
            // https://github.com/igneous-labs/inf-jup-interface/pull/4/changes#r2876051458
            fee_amount: 0,
            fee_mint: Pubkey::new_from_array(quote.out_mint),
            fee_pct: Decimal::ZERO,
        })
    }

    fn get_swap_and_account_metas(
        &self,
        SwapParams {
            swap_mode,
            in_amount,
            out_amount,
            source_mint,
            destination_mint,
            source_token_account,
            destination_token_account,
            token_transfer_authority,
            ..
        }: &SwapParams,
    ) -> Result<SwapAndAccountMetas> {
        let limit_ty = swap_mode_to_trade_limit_ty(*swap_mode);
        let (amt, limit) = match limit_ty {
            TradeLimitTy::ExactIn(_) => (in_amount, out_amount),
            TradeLimitTy::ExactOut(_) => (out_amount, in_amount),
        };
        let args = TradeIxArgs {
            amt: *amt,
            limit: *limit,
            mints: &Pair {
                inp: source_mint.as_array(),
                out: destination_mint.as_array(),
            },
            signer: token_transfer_authority.as_array(),
            token_accs: &Pair {
                inp: source_token_account.as_array(),
                out: destination_token_account.as_array(),
            },
        };
        let ix = self.inner.trade_ix(&args, limit_ty).map_err(FmtErr)?;
        let mut account_metas = vec![AccountMeta::new_readonly(self.program_id(), false)];
        let full = match ix {
            Trade::ExactIn(ix) => {
                account_metas.extend(keys_writable_to_jup_metas(
                    swap_exact_in_v2_ix_keys_owned(&ix.accs).seq(),
                    swap_exact_in_v2_ix_is_writer(&ix.accs).seq(),
                ));
                ix.to_full()
            }
            Trade::ExactOut(ix) => {
                account_metas.extend(keys_writable_to_jup_metas(
                    swap_exact_out_v2_ix_keys_owned(&ix.accs).seq(),
                    swap_exact_out_v2_ix_is_writer(&ix.accs).seq(),
                ));
                ix.to_full()
            }
        };
        // `u32::MAX` represents the controller LP token instead of an LST-list index:
        // - source/input is `u32::MAX`: LP -> LST (remove liquidity)
        // - destination/output is `u32::MAX`: LST -> LP (add liquidity)
        Ok(SwapAndAccountMetas {
            swap: Swap::SanctumSV2 {
                src_lst_value_calc_accs: full.inp_lst_value_calc_accs,
                dst_lst_value_calc_accs: full.out_lst_value_calc_accs,
                src_lst_index: full.inp_lst_index,
                dst_lst_index: full.out_lst_index,
            },
            account_metas,
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn supports_exact_out(&self) -> bool {
        true
    }

    fn program_dependencies(&self) -> Vec<(Pubkey, String)> {
        PROGRAM_DEPENDENCIES
            .into_iter()
            .map(|(program_id, label)| (program_id.into(), label.into()))
            .collect()
    }

    fn get_accounts_len(&self) -> usize {
        32
    }
}

pub const PROGRAM_DEPENDENCIES: [([u8; 32], &str); 16] = [
    // SPL
    (inf1_svc_spl_core::keys::spl::POOL_PROG_ID, "spl_stake_pool"),
    (inf1_svc_spl_core::keys::spl::ID, "spl_calculator"),
    // Sanctum SPL
    (
        inf1_svc_spl_core::keys::sanctum_spl::POOL_PROG_ID,
        "sanctum_spl_stake_pool",
    ),
    (
        inf1_svc_spl_core::keys::sanctum_spl::ID,
        "sanctum_spl_calculator",
    ),
    // Sanctum SPL Multi
    (
        inf1_svc_spl_core::keys::sanctum_spl_multi::POOL_PROG_ID,
        "sanctum_spl_multi_stake_pool",
    ),
    (
        inf1_svc_spl_core::keys::sanctum_spl_multi::ID,
        "sanctum_spl_multi_calculator",
    ),
    // marinade
    (inf1_svc_marinade_core::keys::POOL_PROG_ID, "marinade"),
    (inf1_svc_marinade_core::ID, "marinade_calculator"),
    // lido
    (inf1_svc_lido_core::keys::POOL_PROG_ID, "lido"),
    (inf1_svc_lido_core::ID, "lido_calculator"),
    // wSOL
    (inf1_svc_wsol_core::ID, "wsol_calculator"),
    // pricing programs
    (inf1_pp_flatfee_core::ID, "flat_fee_pricing_program"),
    (inf1_pp_flatslab_core::ID, "flat_slab_pricing_program"),
    (inf1_pp_reserve_v2_core::ID, "reserve_v2_pricing_program"),
    // INF pool and calculator used to price INF as an LST
    (inf1_std::inf1_ctl_core::keys::INF_PROGRAM_ID, "inf"),
    (
        inf1_std::inf1_ctl_core::svc::INF_SVC_PROGRAM_ID,
        "inf_calculator",
    ),
];

#[inline]
pub const fn swap_mode_to_trade_limit_ty(sm: SwapMode) -> TradeLimitTy {
    match sm {
        SwapMode::ExactIn => TradeLimitTy::ExactIn(()),
        SwapMode::ExactOut => TradeLimitTy::ExactOut(()),
    }
}

pub fn keys_writable_to_jup_metas<'a>(
    keys: impl Iterator<Item = &'a [u8; 32]>,
    writable: impl Iterator<Item = &'a bool>,
) -> Vec<AccountMeta> {
    keys.zip(writable)
        .map(|(key, writable)| AccountMeta {
            pubkey: Pubkey::new_from_array(*key),
            is_signer: false, // The signer is elevated by the jupiter instruction, otherwise uses shared accounts and elevated internally before CPI
            is_writable: *writable,
        })
        .collect()
}
