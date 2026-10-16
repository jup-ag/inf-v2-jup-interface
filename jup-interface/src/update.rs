use std::sync::atomic::Ordering;

use inf1_std::{
    inf1_svc_ag_std::inf1_svc_inf_std::SYSVAR_CLOCK,
    update::{Account, UpdateMap},
};
use jupiter_amm_interface::{AccountMap, ClockRef};
use solana_pubkey::Pubkey;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct AccountRef<'a>(pub &'a solana_account::Account);

impl Account for AccountRef<'_> {
    #[inline]
    fn data(&self) -> &[u8] {
        &self.0.data
    }

    #[inline]
    fn lamports(&self) -> u64 {
        self.0.lamports
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccountMapRef<'a> {
    pub account_map: &'a AccountMap,
    pub clock: &'a solana_account::Account,
}

impl UpdateMap for AccountMapRef<'_> {
    type Account<'acc>
        = AccountRef<'acc>
    where
        Self: 'acc;

    #[inline]
    fn get_account(&self, pk: &[u8; 32]) -> Option<Self::Account<'_>> {
        match pk {
            &SYSVAR_CLOCK => Some(self.clock),
            pk => self.account_map.get(&Pubkey::new_from_array(*pk)),
        }
        .map(AccountRef)
    }
}

pub fn sim_clock(clock_ref: &ClockRef) -> solana_account::Account {
    solana_account::Account {
        lamports: 1_169_280,
        data: ser_clock(clock_ref).into(),
        owner: Pubkey::from_str_const("Sysvar1111111111111111111111111111111111111"),
        executable: false,
        rent_epoch: u64::MAX,
    }
}

const CLOCK_SIZE: usize = 40;

fn ser_clock(c: &ClockRef) -> [u8; CLOCK_SIZE] {
    const A: usize = CLOCK_SIZE;

    let mut d = [0u8; A];

    d = caba::<A, 0, 8>(d, &c.slot.load(Ordering::Relaxed).to_le_bytes());
    d = caba::<A, 8, 8>(
        d,
        &c.epoch_start_timestamp
            .load(Ordering::Relaxed)
            .to_le_bytes(),
    );
    d = caba::<A, 16, 8>(d, &c.epoch.load(Ordering::Relaxed).to_le_bytes());
    d = caba::<A, 24, 8>(
        d,
        &c.leader_schedule_epoch
            .load(Ordering::Relaxed)
            .to_le_bytes(),
    );
    d = caba::<A, 32, 8>(d, &c.unix_timestamp.load(Ordering::Relaxed).to_le_bytes());

    d
}

/// caba = `const_assign_byte_array`
pub const fn caba<const A: usize, const START: usize, const LEN: usize>(
    mut arr: [u8; A],
    val: &[u8; LEN],
) -> [u8; A] {
    const {
        assert!(START + LEN <= A);
    }

    let mut i = 0;
    while i < LEN {
        arr[START + i] = val[i];
        i += 1;
    }
    arr
}
