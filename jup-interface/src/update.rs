use inf1_std::update::{Account, UpdateMap};
use jupiter_amm_interface::AccountMap;
use solana_account::Account as SolanaAccount;
use solana_pubkey::Pubkey;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct AccountRef<'a>(pub &'a SolanaAccount);

impl Account for AccountRef<'_> {
    #[inline]
    fn data(&self) -> &[u8] {
        &self.0.data
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct AccountMapRef<'a>(pub &'a AccountMap);

impl UpdateMap for AccountMapRef<'_> {
    type Account<'acc>
        = AccountRef<'acc>
    where
        Self: 'acc;

    #[inline]
    fn get_account(&self, pk: &[u8; 32]) -> Option<Self::Account<'_>> {
        self.0.get(&Pubkey::new_from_array(*pk)).map(AccountRef)
    }
}
