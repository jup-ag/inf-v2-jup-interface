use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

use inf1_std::{
    err::{InfErr, NotEnoughLiquidityErr},
    inf1_pp_ag_std::{
        inf1_pp_flatfee_std::{traits::FlatFeePricingColErr, update::FlatFeePricingUpdateErr},
        inf1_pp_flatslab_std::{
            traits::FlatSlabPricingColErr, typedefs::MintNotFoundErr,
            update::FlatSlabPricingUpdateErr,
        },
        inf1_pp_reserve_v2_std::{
            errs::{ReserveV2ProgramErr, SameMintErr},
            pricing::ReserveV2Swap,
            typedefs::MintNotFoundErr as ReserveV2MintNotFoundErr,
            update::ReserveV2PricingUpdateErr,
            ReserveV2PricingColErr,
        },
        pricing::PricingAgErr,
        update::UpdatePpErr,
        PricingAg, PricingProgAgErr,
    },
    inf1_svc_ag_std::{
        calc::SvcCalcAgErr,
        update::{
            InfExtUpdateErr, InfUpdateErr, LidoUpdateErr, MarinadeUpdateErr, SplUpdateErr,
            UpdateSvcErr,
        },
        SvcAg,
    },
    quote::{rebalance::RebalanceQuoteErr, swap::err::QuoteErr},
    update::UpdateErr,
};
use solana_pubkey::Pubkey;

/// Newtype wrapper to enable pretty-printing of pubkeys
#[repr(transparent)]
pub struct FmtErr<E>(pub E);

impl<E: Debug> Debug for FmtErr<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for FmtErr<InfErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            InfErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
            InfErr::MissingAcc { pk } => {
                f.write_fmt(format_args!("MissingAcc: {}", Pubkey::new_from_array(pk)))
            }
            InfErr::MissingReserves { mint } => f.write_fmt(format_args!(
                "MissingReserves: {}",
                Pubkey::new_from_array(mint)
            )),
            InfErr::MissingSplData { mint } => f.write_fmt(format_args!(
                "MissingSplData: {}",
                Pubkey::new_from_array(mint)
            )),
            InfErr::MissingSvcData { mint } => f.write_fmt(format_args!(
                "MissingSvcData: {}",
                Pubkey::new_from_array(mint)
            )),
            InfErr::UnknownPp { pp_prog_id } => f.write_fmt(format_args!(
                "UnknownPp: {}",
                Pubkey::new_from_array(pp_prog_id)
            )),
            InfErr::UnknownSvc { svc_prog_id } => f.write_fmt(format_args!(
                "UnknownSvc: {}",
                Pubkey::new_from_array(svc_prog_id)
            )),
            InfErr::UnsupportedMint { mint } => f.write_fmt(format_args!(
                "UnsupportedMint: {}",
                Pubkey::new_from_array(mint)
            )),

            // inner wrapper
            InfErr::PricingProg(e) => Display::fmt(&FmtErr(e), f),
            InfErr::RebalanceQuote(e) => Display::fmt(&FmtErr(e), f),
            InfErr::SwapQuote(e) => Display::fmt(&FmtErr(e), f),
            InfErr::UpdatePp(e) => Display::fmt(&FmtErr(e), f),
            InfErr::UpdateSvc(e) => Display::fmt(&FmtErr(e), f),

            // dont need to wrap inner in FmtErr since these errs do not
            // contain pubkey fields
            InfErr::Ctl(e) => Display::fmt(&e, f),
            InfErr::NoValidPda => Display::fmt(&self.0, f),
        }
    }
}

impl Error for FmtErr<InfErr> {}

impl Display for FmtErr<UpdateErr<InfErr>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            UpdateErr::AccMissing { pk } => {
                f.write_fmt(format_args!("MissingAcc: {}", Pubkey::new_from_array(pk)))
            }
            UpdateErr::Inner(e) => Display::fmt(&FmtErr(e), f),
        }
    }
}

impl Error for FmtErr<UpdateErr<InfErr>> {}

impl Display for FmtErr<PricingProgAgErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            PricingAg::FlatFee(e) => Display::fmt(&FmtErr(e), f),
            PricingAg::FlatSlab(e) => Display::fmt(&FmtErr(e), f),
            PricingAg::ReserveV2(e) => Display::fmt(&FmtErr(e), f),
        }
    }
}

impl Display for FmtErr<FlatFeePricingColErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            FlatFeePricingColErr::FeeAccountMissing { mint } => f.write_fmt(format_args!(
                "FeeAccountMissing: {}",
                Pubkey::new_from_array(mint)
            )),
            FlatFeePricingColErr::ProgramStateMissing => Display::fmt(&self.0, f),
        }
    }
}

impl Display for FmtErr<FlatSlabPricingColErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            FlatSlabPricingColErr::MintNotFound(MintNotFoundErr { mint, .. }) => f.write_fmt(
                format_args!("MintNotFound: {}", Pubkey::new_from_array(mint)),
            ),
        }
    }
}

impl Display for FmtErr<MintNotFoundErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let MintNotFoundErr { mint, .. } = self.0;
        f.write_fmt(format_args!(
            "MintNotFound: {}",
            Pubkey::new_from_array(mint)
        ))
    }
}

impl Display for FmtErr<ReserveV2PricingColErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            ReserveV2PricingColErr::Program(e) => Display::fmt(&FmtErr(e), f),
            ReserveV2PricingColErr::Ctl(e) => Display::fmt(&e, f),
            ReserveV2PricingColErr::NotUpdated => Display::fmt(&self.0, f),
        }
    }
}

impl Display for FmtErr<ReserveV2ProgramErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            ReserveV2ProgramErr::MintNotFound(ReserveV2MintNotFoundErr { mint, .. }) => f
                .write_fmt(format_args!(
                    "MintNotFound: {}",
                    Pubkey::new_from_array(mint)
                )),
            ReserveV2ProgramErr::SameMint(SameMintErr { mint }) => {
                f.write_fmt(format_args!("SameMint: {}", Pubkey::new_from_array(mint)))
            }
            ReserveV2ProgramErr::CantRemoveRequiredMint
            | ReserveV2ProgramErr::FeeNanosOutOfRange(_)
            | ReserveV2ProgramErr::MathOverflow
            | ReserveV2ProgramErr::NegativeBandDelta
            | ReserveV2ProgramErr::OverCap(_)
            | ReserveV2ProgramErr::ThresholdNanosOutOfRange(_)
            | ReserveV2ProgramErr::UnsupportedDeprecatedInstruction
            | ReserveV2ProgramErr::WsolBalanceGtPoolSolValue(_)
            | ReserveV2ProgramErr::ZeroRetainedValue
            | ReserveV2ProgramErr::ZeroPoolSolValue => Display::fmt(&self.0, f),
        }
    }
}

impl Display for FmtErr<PricingAgErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            PricingAg::FlatFee(e) => Display::fmt(&e, f),
            PricingAg::FlatSlab(e) => Display::fmt(&e, f),
            PricingAg::ReserveV2(e) => Display::fmt(&FmtErr(e), f),
        }
    }
}

impl Display for FmtErr<ReserveV2Swap<ReserveV2ProgramErr, ReserveV2ProgramErr>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            ReserveV2Swap::Flat(e) | ReserveV2Swap::RangeOut(e) => Display::fmt(&FmtErr(e), f),
        }
    }
}

impl Display for FmtErr<RebalanceQuoteErr<SvcCalcAgErr, SvcCalcAgErr>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            RebalanceQuoteErr::NotEnoughLiquidity(e) => Display::fmt(&FmtErr(e), f),
            // all variants here dont have any fields that require formatting
            RebalanceQuoteErr::InpCalc(_)
            | RebalanceQuoteErr::OutCalc(_)
            | RebalanceQuoteErr::Overflow => Display::fmt(&self.0, f),
        }
    }
}

impl Display for FmtErr<QuoteErr<SvcCalcAgErr, SvcCalcAgErr, PricingAgErr>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            QuoteErr::NotEnoughLiquidity(e) => Display::fmt(&FmtErr(e), f),
            QuoteErr::Pricing(e) => Display::fmt(&FmtErr(e), f),
            // all variants here dont have any fields that require formatting
            QuoteErr::InpCalc(_)
            | QuoteErr::InpDisabled
            | QuoteErr::OutCalc(_)
            | QuoteErr::PoolLoss
            | QuoteErr::ZeroValue => Display::fmt(&self.0, f),
        }
    }
}

impl Display for FmtErr<NotEnoughLiquidityErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "NotEnoughLiquidity. Required: {}. Available: {}",
            self.0.required, self.0.available
        ))
    }
}

impl Display for FmtErr<UpdatePpErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            PricingAg::FlatFee(e) => Display::fmt(&FmtErr(e), f),
            PricingAg::FlatSlab(e) => Display::fmt(&FmtErr(e), f),
            PricingAg::ReserveV2(e) => Display::fmt(&FmtErr(e), f),
        }
    }
}

impl Display for FmtErr<FlatFeePricingUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            FlatFeePricingUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<FlatSlabPricingUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            FlatSlabPricingUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<ReserveV2PricingUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            ReserveV2PricingUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<UpdateSvcErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            SvcAg::Inf(e) => Display::fmt(&FmtErr(e), f),
            SvcAg::InfExt(e) => Display::fmt(&FmtErr(e), f),
            SvcAg::Lido(e) => Display::fmt(&FmtErr(e), f),
            SvcAg::Marinade(e) => Display::fmt(&FmtErr(e), f),
            SvcAg::SanctumSpl(e) | SvcAg::SanctumSplMulti(e) | SvcAg::Spl(e) => {
                Display::fmt(&FmtErr(e), f)
            }
            SvcAg::Wsol(_infallible) => unreachable!(),
        }
    }
}

impl Display for FmtErr<InfUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            InfUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<InfExtUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            InfExtUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
            InfExtUpdateErr::Ctl(e) => Display::fmt(&e, f),
        }
    }
}

impl Display for FmtErr<LidoUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            LidoUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<MarinadeUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            MarinadeUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}

impl Display for FmtErr<SplUpdateErr> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            SplUpdateErr::AccDeser { pk } => {
                f.write_fmt(format_args!("AccDeser: {}", Pubkey::new_from_array(pk)))
            }
        }
    }
}
