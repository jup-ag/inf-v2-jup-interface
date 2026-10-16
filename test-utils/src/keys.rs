use generic_array_struct::generic_array_struct;
use solana_pubkey::Pubkey;

#[generic_array_struct(builder pub)]
pub struct ConstKeys<T> {
    // sysvars and cluster consts
    pub sysvar_owner: T,
    pub sysvar_clock: T,
    pub bpf_loader_upgradeable: T,
    // programs
    pub inf_ctl_prog: T,
    pub reserve_v2_prog: T,
    pub reserve_v2_pp_prog: T,
    pub inf_svc_prog: T,
    pub flatslab_pp_prog: T,
    pub lido_calc_prog: T,
    pub marinade_calc_prog: T,
    pub sanctum_spl_multi_calc_prog: T,
    pub wsol_calc_prog: T,
    pub lido_prog: T,
    pub lido_progdata: T,
    pub marinade_prog: T,
    pub marinade_progdata: T,
    pub sanctum_spl_multi_prog: T,
    pub sanctum_spl_multi_progdata: T,
    // stake pools
    pub jupsol_pool: T,
    // mints
    pub jupsol_mint: T,
    // there are other fixture keys that are not programmatically used
    // or imported from other sources
}

impl<T: Copy> ConstKeys<T> {
    #[inline]
    pub const fn memset(v: T) -> Self {
        Self([v; CONST_KEYS_LEN])
    }
}

pub const CONST_KEYS_STR: ConstKeys<&'static str> = ConstKeys::memset("")
    .const_with_sysvar_owner("Sysvar1111111111111111111111111111111111111")
    .const_with_sysvar_clock("SysvarC1ock11111111111111111111111111111111")
    .const_with_bpf_loader_upgradeable("BPFLoaderUpgradeab1e11111111111111111111111")
    .const_with_inf_ctl_prog("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx")
    .const_with_reserve_v2_prog("un27kVAKYscfzvrkNeYkNZ74tW9o4txuArAweftjakw")
    .const_with_reserve_v2_pp_prog("uppoVuoFZuXisHkrxCU96VvNibU6vzxkEpeH3WbmnEn")
    .const_with_inf_svc_prog("1nf7dspGYz1CTALJbtNgjvcSYWiFz5N3c2EuUZLSWCL")
    .const_with_flatslab_pp_prog("s1b6NRXj6ygNu1QMKXh2H9LUR2aPApAAm1UQ2DjdhNV")
    .const_with_lido_calc_prog("1idUSy4MGGKyKhvjSnGZ6Zc7Q4eKQcibym4BkEEw9KR")
    .const_with_marinade_calc_prog("mare3SCyfZkAndpBRBeonETmkCCB3TJTTrz8ZN2dnhP")
    .const_with_sanctum_spl_multi_calc_prog("ssmbu3KZxgonUtjEMCKspZzxvUQCxAFnyh1rcHUeEDo")
    .const_with_wsol_calc_prog("wsoGmxQLSvwWpuaidCApxN5kEowLe2HLQLJhCQnj4bE")
    .const_with_lido_prog("CrX7kMhLC3cSsXJdT7JDgqrRVWGnUpX3gfEfxxU2NVLi")
    .const_with_lido_progdata("CHZNLhDXKrsXBmmv947RFciquwBsn2NdABmhpxoX3wgZ")
    .const_with_marinade_prog("MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD")
    .const_with_marinade_progdata("4PQH9YmfuKrVyZaibkLYpJZPv2FPaybhq2GAuBcWMSBf")
    .const_with_sanctum_spl_multi_prog("SPMBzsVUuoHA4Jm6KunbsotaahvVikZs1JyTW6iJvbn")
    .const_with_sanctum_spl_multi_progdata("HxBTMuB7cFBPVWVJjTi9iBF8MPd7mfY1QnrrWfLAySFd")
    .const_with_jupsol_pool("8VpRhuxa7sUUepdY3kQiTmX9rS5vx4WgaXiAnXq4KCtr")
    .const_with_jupsol_mint("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v");

pub const CONST_PUBKEYS: ConstKeys<Pubkey> = {
    let mut res = ConstKeys::memset(Pubkey::new_from_array([0; 32]));
    let mut i = 0;
    while i < CONST_KEYS_LEN {
        res.0[i] = Pubkey::from_str_const(CONST_KEYS_STR.0[i]);
        i += 1;
    }
    res
};
