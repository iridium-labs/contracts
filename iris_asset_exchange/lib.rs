#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;

    #[ink(extension = 0, returns_result = false)]
    fn transfer_asset(caller: ink_env::AccountId, target: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];

    #[ink(extension = 1, returns_result = false)]
    fn mint(caller: ink_env::AccountId, target: ink_env::AccountId, asset_id: u32, amount: u64) -> [u8; 32];

    #[ink(extension = 2, returns_result = false)]
    fn lock(amount: u64) -> [u8; 32];

    #[ink(extension = 3, returns_result = false)]
    fn unlock_and_transfer(target: ink_env::AccountId) -> [u8; 32];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailTransferAsset,
    FailMintAssets,
    FailLockCurrency,
    FailUnlockCurrency,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Err(Self::FailTransferAsset),
            1 => Err(Self::FailMintAssets),
            2 => Err(Self::FailLockCurrency),
            3 => Err(Self::FailUnlockCurrency),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}

impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = Iris;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod iris_asset_exchange {
    // use ink_lang as ink;
    use super::IrisErr;
    use ink_storage::traits::SpreadAllocate;

    /// Defines the storage of our contract.
    ///
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct IrisAssetExchange {
        /// maps the owner of a token sale to the asset id and asking price 
        registry: ink_storage::Mapping<(AccountId, u32), u64>,
    }

    #[ink(event)]
    pub struct AssetTransferSuccess { }

    #[ink(event)]
    pub struct NewTokenSaleSuccess { }

    #[ink(event)]
    pub struct AssetNotRegistered { }

    impl IrisAssetExchange {

        /// build a new  Iris Asset Exchange
        #[ink(constructor, payable)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|_| {})
        }

        /// Default constructor
        #[ink(constructor, payable)]
        pub fn default() -> Self {
            Self::new()
        }

        /// Provide pricing for a static amount of assets.
        /// 
        /// This function mints new assets from an asset class owned by the caller 
        /// and assigns them to the contract address. It adds an item to the exchange's registry,
        /// associating the asset id with the price determined by the caller.
        /// 
        /// * `asset_id`: An asset_id associated with an owned asset class
        /// * `amount`: The amount of assets that will be minted and provisioned to the exchange
        /// * `price`: The price (in OBOL) of each token
        /// 
         #[ink(message)]
         pub fn publish_sale(&mut self, asset_id: u32, amount: u64, price: u64) {
             let caller = self.env().caller();
             self.env()
                 .extension()
                 .mint(
                     caller, self.env().account_id(), asset_id, amount,
                 ).map_err(|_| {});
            self.registry.insert((&caller, &asset_id), &price);
             self.env().emit_event(AssetTransferSuccess { });
         }

        /// Purchase assets from the exchange.
        /// 
        /// This function performs the following process:
        /// 1. lock price*amount tokens
        /// 2. Transfer the asset from the contract account to the caller
        /// 3. unlock the locked tokens from (1) and transfer to the owner of the asset class
        /// 
        /// * `owner`: The owner of the asset class from which the asset to be purchased was minted
        /// * `asset_id`: The id of the owned asset class
        /// * `amount`: The amount of assets to purchase
        /// 
        #[ink(message)]
        pub fn purchase_tokens(&mut self, owner: AccountId, asset_id: u32, amount: u64) {
            let caller = self.env().caller();
            // calculate total cost
            if let Some(price) = self.registry.get((&owner, &asset_id)) {
                let total_cost = amount * price;
                // caller locks total_cost
                self.env().extension().lock(total_cost).map_err(|_| {});
                // contract grants tokens to caller
                self.env()
                    .extension()
                    .transfer_asset(
                        self.env().account_id(), caller, asset_id, amount,
                    ).map_err(|_| {});
                // caller send tokens to owner
                self.env().extension().unlock_and_transfer(owner).map_err(|_| {});
                self.env().emit_event(AssetTransferSuccess { });
            } else {
                self.env().emit_event(AssetNotRegistered { });
            }
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {
            let _iris_asset_exchange = IrisAssetExchange::default();
            // assert_eq!(iris_asset_exchange.registry.len(), [0; 32]);
        }

        // #[ink::test]
        // fn chain_extension_works() {
        //     // given
        //     struct MockedExtension;
        //     impl ink_env::test::ChainExtension for MockedExtension {
        //         /// The static function id of the chain extension.
        //         fn func_id(&self) -> u32 {
        //             1101
        //         }

        //         /// The chain extension is called with the given input.
        //         ///
        //         /// Returns an error code and may fill the `output` buffer with a
        //         /// SCALE encoded result. The error code is taken from the
        //         /// `ink_env::chain_extension::FromStatusCode` implementation for
        //         /// `RandomReadErr`.
        //         fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
        //             let ret: [u8; 32] = [1; 32];
        //             scale::Encode::encode_to(&ret, output);
        //             0
        //         }
        //     }
        //     ink_env::test::register_chain_extension(MockedExtension);
        //     let mut rand_extension = RandExtension::default();
        //     assert_eq!(rand_extension.get(), [0; 32]);

        //     // when
        //     rand_extension.update([0_u8; 32]).expect("update must work");

        //     // then
        //     assert_eq!(rand_extension.get(), [1; 32]);
        // }
    }
}
