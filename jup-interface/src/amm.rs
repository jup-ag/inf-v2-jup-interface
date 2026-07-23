//! Reference-only: NOT compiled (no `mod` decl in lib.rs).
//! The live Jupiter `Amm` impl is owned by jupiter-core (`src/amms/inf_v2_amm.rs`)
//! so this crate stays free of any published `jupiter-amm-interface` version.

use std::{
    collections::HashMap,
    iter::once,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

pub use ::inf1_std;
use ::sanctum_lst_list::{PoolInfo, SanctumLst};
use anyhow::{anyhow, Context, Result};
use inf1_std::{
    err::InfErr,
    inf1_ctl_core::{
        accounts::lst_state_list::LstStatePackedList, keys::CONST_KEYS_OWNED,
        typedefs::lst_state::LstState,
    },
    inf1_pp_ag_std::{
        inf1_pp_flatfee_core,
        update::{all::AccountsToUpdateAll, UpdatePricingProg},
    },
    inf1_pp_core::pair::Pair,
    inf1_svc_ag_std::{
        inf1_svc_lido_core::{self, calc::LidoCalcErr, solido_legacy_core::SYSVAR_CLOCK},
        inf1_svc_marinade_core,
        inf1_svc_spl_core::{self, calc::SplCalcErr},
        inf1_svc_wsol_core,
        update::UpdateSvc,
        SvcAg,
    },
    instructions::swap::v2::{
        exact_in::{swap_exact_in_v2_ix_is_writer, swap_exact_in_v2_ix_keys_owned},
        exact_out::{swap_exact_out_v2_ix_is_writer, swap_exact_out_v2_ix_keys_owned},
    },
    pda::CONST_PDA_KEYS_OWNED,
    quote::swap::err::QuoteErr,
    trade::{instruction::TradeIxArgs, Trade, TradeLimitTy},
    update::UpdateErr,
    InfStd,
};
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas,
    SwapMode, SwapParams,
};
use rust_decimal::Decimal;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::{
    clock::is_epoch_affected_lst_mint,
    consts::{DEFAULT_MAINNET_POOL, LABEL},
    err::FmtErr,
};
use crate::update::AccountMapRef;

// Note on Clock hax:
// Because `Clock` is a special-case account, and because it's only used
// by Lido and Spl SolValCalcs to check current epoch to filter out unexecutable quoting rn:
// - we exclude it from all update accounts
// - update procedures use the `_no_clock()` variants that dont
//   update clock data and hence dont rely on clock acc being in AccountMap
// - `current_epoch=0` on all the SolValCalc structs so that quoting will never
//   fail due to the underlying stake pool not being updated for the epoch
// - we only check for underlying stake pool not being updated for the epoch
//   during the quoting procedure to determine whether to return err
fn build_spl_lsts() -> HashMap<[u8; 32], [u8; 32]> {
    sanctum_lst_list::load_sanctum_lst_list()
        .into_iter()
        .filter_map(|SanctumLst { mint, pool, .. }| {
            let stake_pool_address = match pool {
                PoolInfo::Lido
                | PoolInfo::Marinade
                | PoolInfo::ReservePool
                | PoolInfo::SPool(_) => return None,
                PoolInfo::SanctumSpl(spl_pool_accounts)
                | PoolInfo::Spl(spl_pool_accounts)
                | PoolInfo::SanctumSplMulti(spl_pool_accounts) => spl_pool_accounts.pool.to_bytes(),
            };
            Some((mint.to_bytes(), stake_pool_address))
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct InfAmm {
    pub inner: InfStd,
    pub current_epoch: Arc<AtomicU64>,
}

impl InfAmm {
    pub const PROGRAM_ID: Pubkey = INF_PROGRAM_ID;

    pub fn new(
        keyed_account: &KeyedAccount,
        amm_context: &AmmContext,
        spl_lsts: HashMap<[u8; 32], [u8; 32]>,
    ) -> Result<Self> {
        if *keyed_account.key.as_array() != *CONST_PDA_KEYS_OWNED.lst_state_list() {
            return Err(anyhow!("Incorrect LST state list keyed_account"));
        }

        let mut res = Self {
            inner: InfStd::new(
                Some(*CONST_KEYS_OWNED.program()),
                DEFAULT_MAINNET_POOL,
                keyed_account.account.data.clone().into_boxed_slice(),
                None,
                None,
                Default::default(),
                Default::default(),
                spl_lsts,
                find_pda,
                create_raw_pda,
            )
            .map_err(FmtErr)?,
            current_epoch: amm_context.clock_ref.epoch.clone(),
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
        LABEL.to_owned()
    }

    fn program_id(&self) -> Pubkey {
        INF_PROGRAM_ID
    }

    /// S Pools are 1 per program, so just use program ID as key
    fn key(&self) -> Pubkey {
        INF_LST_LIST_ID
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
            .unwrap_or_default()
            .iter()
            .map(|l| l.into_lst_state());
        [
            *CONST_PDA_KEYS_OWNED.pool_state(),
            *CONST_PDA_KEYS_OWNED.lst_state_list(),
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
                    // ignore err here, some LSTs may not have their
                    // sol val calc accounts fetched yet.
                    //
                    // update() should call `try_get_or_init_lst_svc_mut`
                    // which will make it no longer err for the next update cycle
                    self.inner
                        .accounts_to_update_lst(&lst_state)
                        .ok()
                        .map(|iter| iter.filter(|pk| *pk != SYSVAR_CLOCK))
                })
                .flatten(),
        )
        .map(Pubkey::new_from_array)
        .collect()
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let fetched = AccountMapRef(account_map);
        self.inner.update_pool(fetched).map_err(FmtErr)?;
        self.inner.update_lst_state_list(fetched).map_err(FmtErr)?;
        self.inner.update_lp_token_supply(fetched).map_err(FmtErr)?;

        let inf1_std::Inf {
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
                pk: *CONST_PDA_KEYS_OWNED.lst_state_list(),
            }))?
            .0
            .iter()
            .map(|s| s.into_lst_state());

        pricing.update_all(
            all_lst_states.clone().map(|LstState { mint, .. }| mint),
            fetched,
        )?;

        all_lst_states
            .try_for_each(|lst_state| {
                inf1_std::InfStd::update_lst_reserves(
                    lst_reserves,
                    create_pda as &_,
                    CONST_PDA_KEYS_OWNED.pool_state(),
                    &lst_state,
                    fetched,
                )?;

                let calc = match inf1_std::InfStd::try_get_or_init_lst_svc_static(
                    lst_calcs, spl_lsts, &lst_state,
                ) {
                    Ok(calc) => calc,
                    Err(error) => {
                        if matches!(error, InfErr::MissingSplData { .. }) {
                            lst_calcs.remove(&lst_state.mint);
                            return Ok(());
                        } else {
                            return Err(UpdateErr::Inner(error));
                        }
                    }
                };

                match &mut calc.0 {
                    // omit clock for these variants
                    SvcAg::Inf(c) => c
                        .update_svc(fetched)
                        .map_err(|e| e.map_inner(SvcAg::Inf).map_inner(InfErr::UpdateSvc)),
                    SvcAg::Lido(c) => c
                        .update_svc_no_clock(fetched)
                        .map_err(|e| e.map_inner(SvcAg::Lido).map_inner(InfErr::UpdateSvc)),
                    SvcAg::SanctumSpl(c) => c
                        .update_svc_no_clock(fetched)
                        .map_err(|e| e.map_inner(SvcAg::SanctumSpl).map_inner(InfErr::UpdateSvc)),
                    SvcAg::SanctumSplMulti(c) => c.update_svc_no_clock(fetched).map_err(|e| {
                        e.map_inner(SvcAg::SanctumSplMulti)
                            .map_inner(InfErr::UpdateSvc)
                    }),
                    SvcAg::Spl(c) => c
                        .update_svc_no_clock(fetched)
                        .map_err(|e| e.map_inner(SvcAg::Spl).map_inner(InfErr::UpdateSvc)),
                    // following variants unaffected by clock
                    SvcAg::Marinade(c) => c
                        .update_svc(fetched)
                        .map_err(|e| e.map_inner(SvcAg::Marinade).map_inner(InfErr::UpdateSvc)),
                    SvcAg::Wsol(c) => c
                        .update_svc(fetched)
                        .map_err(|e| e.map_inner(SvcAg::Wsol).map_inner(InfErr::UpdateSvc)),
                }
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
        // clock special-case handling:
        // early return err if any of the mints are
        // epoch affected and epoch conditions dont hold
        for mint in [input_mint, output_mint] {
            let mint = mint.as_array();
            if !is_epoch_affected_lst_mint(mint) {
                continue;
            }

            // since INF is not clock affected, we dont need to
            // worry about try_get_lst_svc() failing for it.
            // In future vers, INF will also have its own sol val calc anyway.
            match self
                .inner
                .try_get_lst_svc(mint)
                .map_err(FmtErr)?
                .as_sol_val_calc()
            {
                Some(c) => match c {
                    SvcAg::Inf(_) | SvcAg::Marinade(_) | SvcAg::Wsol(_) => continue,
                    // kinda sloppy, but if NotUpdated err encountered, just return it under
                    // QuoteErr::InpCalc instead of determining what kind of swap and
                    // what position the affected mint was in
                    SvcAg::Lido(c) => {
                        if c.exchange_rate.computed_in_epoch
                            < self.current_epoch.load(Ordering::Relaxed)
                        {
                            return Err(FmtErr(InfErr::SwapQuote(QuoteErr::InpCalc(SvcAg::Lido(
                                LidoCalcErr::NotUpdated,
                            ))))
                            .into());
                        }
                    }
                    SvcAg::SanctumSpl(c) | SvcAg::SanctumSplMulti(c) | SvcAg::Spl(c) => {
                        if c.last_update_epoch < self.current_epoch.load(Ordering::Relaxed) {
                            return Err(FmtErr(InfErr::SwapQuote(QuoteErr::InpCalc(SvcAg::Spl(
                                SplCalcErr::NotUpdated,
                            ))))
                            .into());
                        }
                    }
                },
                None => return Err(FmtErr(InfErr::MissingSvcData { mint: *mint }).into()),
            }
        }

        let quote = self
            .inner
            .quote_trade(
                &Pair {
                    inp: input_mint.as_array(),
                    out: output_mint.as_array(),
                },
                *amount,
                0,
                swap_mode_to_trade_limit_ty(*swap_mode),
            )
            .map_err(FmtErr)?;

        to_jup_quote(quote)
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
        let mut account_metas = vec![AccountMeta::new_readonly(INF_PROGRAM_ID, false)];

        match ix {
            Trade::ExactIn(ix) => {
                let a = ix.to_full();
                account_metas.extend(keys_writable_to_jup_metas(
                    swap_exact_in_v2_ix_keys_owned(&ix.accs).seq(),
                    swap_exact_in_v2_ix_is_writer(&ix.accs).seq(),
                ));
                Ok(SwapAndAccountMetas {
                    swap: Swap::SanctumS {
                        src_lst_value_calc_accs: a.inp_lst_value_calc_accs,
                        dst_lst_value_calc_accs: a.out_lst_value_calc_accs,
                        src_lst_index: a.inp_lst_index,
                        dst_lst_index: a.out_lst_index,
                    },
                    account_metas,
                })
            }
            Trade::ExactOut(ix) => {
                let a = ix.to_full();
                account_metas.extend(keys_writable_to_jup_metas(
                    swap_exact_out_v2_ix_keys_owned(&ix.accs).seq(),
                    swap_exact_out_v2_ix_is_writer(&ix.accs).seq(),
                ));
                Ok(SwapAndAccountMetas {
                    swap: Swap::SanctumS {
                        src_lst_value_calc_accs: a.inp_lst_value_calc_accs,
                        dst_lst_value_calc_accs: a.out_lst_value_calc_accs,
                        src_lst_index: a.inp_lst_index,
                        dst_lst_index: a.out_lst_index,
                    },
                    account_metas,
                })
            }
        }
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn supports_exact_out(&self) -> bool {
        // Because AddLiquidity and RemoveLiquidity do not support exact out,
        // this only reflects the swap path.
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

pub const PROGRAM_DEPENDENCIES: [([u8; 32], &str); 12] = [
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
    // pricing program
    (inf1_pp_flatfee_core::ID, "flat_fee_pricing_program"),
];

#[inline]
pub const fn swap_mode_to_trade_limit_ty(sm: SwapMode) -> TradeLimitTy {
    match sm {
        SwapMode::ExactIn => TradeLimitTy::ExactIn(()),
        SwapMode::ExactOut => TradeLimitTy::ExactOut(()),
    }
}

#[inline]
pub fn to_jup_quote(
    inf1_std::quote::Quote {
        inp: in_amount,
        out: out_amount,
        fee,
        inp_sol_val,
        inp_mint,
        out_mint: _,
    }: inf1_std::quote::Quote,
) -> Result<Quote, anyhow::Error> {
    let fee_pct_f64 = if inp_sol_val == 0 {
        0.0
    } else {
        (fee as f64) / (inp_sol_val as f64)
    };
    let fee_pct = Decimal::from_f64_retain(fee_pct_f64).ok_or_else(|| anyhow!("Decimal err"))?;
    Ok(Quote {
        in_amount,
        out_amount,
        fee_amount: fee,
        fee_mint: Pubkey::new_from_array(inp_mint),
        fee_pct,
    })
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
