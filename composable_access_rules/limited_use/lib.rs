#![cfg_attr(not(feature = "std"), no_std)]
#![feature(trivial_bounds)]
//!
//! Limited Use Rule
//! 
//! # Goal
//! This contract allows data owners to impose limitations on the number
//! of times an address may use a token to access data associated with the 
//! asset class
//! 
//! # Register
//! 
//! # Execute
//! 
//! 

use ink_env::Environment;
use ink_lang as ink;

/// Functions to interact with the Iris runtime as defined in runtime/src/lib.rs
#[ink::chain_extension]
pub trait Iris {
    type ErrorCode = IrisErr;
    
    #[ink(extension = 6, returns_result = false)]
    fn query_owner(query: ink_env::AccountId, asset_id: u32) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum IrisErr {
    FailQueryOwner,
}

impl ink_env::chain_extension::FromStatusCode for IrisErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            6 => Err(Self::FailQueryOwner),
            _ => panic!("encountered unknown status code {:?}", status_code),
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
mod limited_use_rule {
    use ink_storage::traits::SpreadAllocate;
    use traits::ComposableAccessRule;

    #[ink(event)]
    pub struct RegistrationSuccessful{}

    #[ink(event)]
    pub struct AlreadyRegistered{}

    #[ink(event)]
    pub struct CallerIsNotOwner{}

    #[ink(event)]
    pub struct ExecutionSuccessful{}

    #[ink(event)]
    pub struct ExecutionFailed{}

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct LimitedUseRuleContract {
        limit: u32,
        asset_registry: ink_storage::Mapping<u32, AccountId>,
        usage_counter: ink_storage::Mapping<AccountId, u32>,
    }

    impl LimitedUseRuleContract {
        #[ink(constructor)]
        pub fn new(limit: u32) -> Self {
            if limit <= 0 {
                panic!("limit must be positive");
            }
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                contract.limit = limit;
            })
        }

        fn get_limit(&self) -> u32 {
            self.limit
        }
    }

    impl ComposableAccessRule for LimitedUseRuleContract {

        #[ink(message, payable)]
        fn register(&mut self, asset_id: u32) {
            let caller = self.env().caller();
            if let Some(admin) = self.asset_registry.get(&asset_id) {
                self.env().emit_event(AlreadyRegistered{});
            } else {
                // check that caller is asset owner
                let is_owner = self.env()
                    .extension()
                    .query_owner(caller, asset_id)
                    .map_err(|_| {}).ok();
                match is_owner {
                    Some(true) => {
                        self.asset_registry.insert(&asset_id, &caller);
                        self.env().emit_event(RegistrationSuccessful{});
                    },
                    _ => {
                        self.env().emit_event(CallerIsNotOwner{});
                    }
                }
            }
        }

        #[ink(message, payable)]
        fn execute(&mut self, asset_id: u32) {
            // let caller = self.env().caller();
            // // get count for the asset id
            // let access_limit = self.asset_registry.get(&asset_id);
            // // if let Some(self.usage_counter)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn can_create_new_contract_with_positive_limit() {
            let limit = 10;
            let limited_use_contract = LimitedUseRuleContract::new(limit);
            assert_eq!(limit, limited_use_contract.get_limit());
        }

        // #[ink::test]
        // fn fail_to_create_new_contract_with_zero_limit() {
        //     let limit = 0;
        //     LimitedUseRuleContract::new(limit);
        // }

        /**
         * # Test for the `register` function
         */

        fn setup_register_test(limit: u32, default_account: ink_env::AccountId) -> LimitedUseRuleContract {
            struct MockExtension;
            impl ink_env::test::ChainExtension for MockExtension {
                fn func_id(&self) -> u32 {
                    6
                }
                fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
                    // let ret: AccountId = AccountId::from([0x01; 32]);
                    let ret = true;
                    scale::Encode::encode_to(&ret, output);
                    6
                }
            }

            ink_env::test::register_chain_extension(MockExtension);

            let limited_use_contract = LimitedUseRuleContract::new(limit);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(default_account);
            limited_use_contract
        }

        #[ink::test]
        fn can_register_new_asset_positive_limit() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut limited_use_contract = setup_register_test(1, accounts.alice);
            limited_use_contract.register(1);
            // assert_eq!(accounts.alice, limited_use_contract.asset_registry.get());
        }

        // #[ink::test]
        // fn cant_register_new_asset_with_negative_limit() {

        // }
        
        // #[ink::test]
        // fn cant_register_new_asset_with_zero_limit() {

        // }

        // #[ink::test]
        // fn cant_register_new_asset_when_not_owner() {

        // }
    }
}