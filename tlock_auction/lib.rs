#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink_env::Environment;

#[ink::chain_extension]
pub trait ETF {
    type ErrorCode = EtfErr;

    /// check if a block has been authored in a given slot
    #[ink(extension = 1101, handle_status = false)]
    fn check_slot(slot_id: u32) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum EtfErr {
    FailCheckSlot,
}

impl ink_env::chain_extension::FromStatusCode for EtfErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1101 => Err(Self::FailCheckSlot),
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

    type ChainExtension = ETF;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod tlock_auction {
    use super::EtfErr;
    use ink::storage::Mapping;
    use ink::prelude::vec::Vec;

    use crypto::{
        client::client::{DefaultEtfClient, EtfClient},
        ibe::fullident::BfIbe,
    };
      
    /// represent the asset being auctioned
    #[derive(Debug, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AuctionItem {
        pub name: Vec<u8>,
        pub asset_id: u32,
        pub amount: u8,
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TlockAuction {
        auction_item: AuctionItem,
        /// the slot schedule for this contract
        slot_ids: Vec<u32>,
        threshold: u8,
        proposals: Mapping<AccountId, (Vec<u8>, Vec<u8>, Vec<Vec<u8>>)>, // ciphertext, nonce, capsule
        // deposits: Mapping<AccountId, Balance>,
        /// ink mapping has no support for iteration so we need to loop over this vec to read through the proposals
        /// but maybe could do a struct instead? (acctid, vec, vec, vec)
        participants: Vec<AccountId>,
        // / write the revealed messages
        // revealed_bids: Vec<Vec<u8>>,
        winners: Vec<AccountId>,
        revealed_bids: Vec<Vec<u8>>,
    }

    impl TlockAuction {

        // #[ink(event)]
        // pub struct PublishedBid;

        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor, payable)]
        pub fn new(
            name: Vec<u8>,
            asset_id: u32,
            amount: u8,
            slot_ids: Vec<u32>,
            threshold: u8,
        ) -> Self {
            let auction_item = AuctionItem { name, asset_id, amount };
            let proposals = Mapping::default();
            let participants: Vec<AccountId> = Vec::new();
            let winners: Vec<AccountId> = Vec::new();
            let revealed_bids: Vec<Vec<u8>> = Vec::new();
            // check that they own the asset
            Self {
                auction_item,
                slot_ids,
                threshold,
                proposals,
                participants,
                winners,
                revealed_bids,
            }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(
                b"".to_vec(),
                0, 0,
                Default::default(),
                Default::default(),
            )
        }

        #[ink(message)]
        pub fn get_version(&self) -> Vec<u8> {
            b"0.0.1-dev".to_vec()
        }

        // add your proposal
        // a proposal is a signed, timelocked tx that calls the 'bid' function of this contract
        #[ink(message, payable)]
        pub fn propose(&mut self, ciphertext: Vec<u8>, nonce: Vec<u8>, capsule: Vec<Vec<u8>>) {
            let caller = self.env().caller();
            // if after deadline then return an error
            // TODO: Should there be some validation on owner? this call will fail if the owner is incorrect anyway
            // check if the last slot has passed
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if is_past_deadline {
                // STOP here, return error
            }
            // 2. other checks? [no duplicates, block_list, allow_list]
            // verify min deposit (later)
            // let balance = Self::env().transferred_value();
            // Self::env().transfer(to, balance)?;

            if !self.participants.contains(&caller.clone()) {
                self.participants.push(caller.clone());
            }
            self.proposals.insert(caller, &(ciphertext, nonce, capsule));
            // let _ = self.env().transfer(self.env().account_id(), deposit);
            // emit event here
            // Self::env().emit_event(PublishedBid{});
        }

        #[ink(message)]
        pub fn bid(&mut self, amount: Balance) {
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if is_past_deadline {
                // if before the deadline, return an error
                if self.winners.contains(&self.env().caller()) {
                    // payout amount to owner
                    // self.env().transfer(self.env().account_id(), amount);
                    // owner transfers nft to winner
                } else {
                    // you lost, return deposit 
                }
            } else {
                // return error
            }
        }

        #[ink(message)]
        pub fn complete(&mut self, pp: Vec<u8>, secrets: Vec<Vec<u8>>) {
            let is_past_deadline = self.env()
                .extension()
                .check_slot(self.slot_ids[self.slot_ids.len() - 1]);
            if !is_past_deadline {
                // STOP here, return error
            }
            // 1. ensure past deadline
            self.participants.iter().for_each(|p| {
                self.proposals.get(&p).iter().for_each(|proposal| {
                    let signed_tx = DefaultEtfClient::<BfIbe>::decrypt(
                        pp.clone(), proposal.0.clone(), 
                        proposal.1.clone(), proposal.2.clone(), 
                        secrets.clone(),
                    ).unwrap();
                    // need to decode the tx and get the amount and use it to identify the winner
                    // 1. decode (how?!) + verify
                    // 2. check if winner
                    self.revealed_bids.push(signed_tx);
                });
            });
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crypto::testing::{test_ibe_params, ibe_extract};
        use rand_chacha::{
            rand_core::SeedableRng,
            ChaCha20Rng
        };

        #[ink::test]
        fn default_works() {
            let auction = TlockAuction::default();
            assert_eq!(auction.get_version(), b"0.0.1-dev".to_vec());
        }

        #[ink::test]
        fn can_propose_bid() {
            let slot_ids = vec![vec![1,2,3], vec![2,3,4], vec![3,4,5]];
            let threshold = 2;
            // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test0"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            // setup the auction contract
            let mut auction = TlockAuction::new(b"test1".to_vec(), 1u32, 1u8, slot_ids.clone(), threshold);

            let res = add_bid(slot_ids, threshold, ibe_params.0, ibe_params.1, rng);
            auction.propose(res.0.clone(), res.1.clone(), res.2.clone());

            let participants = auction.participants;
            assert_eq!(participants.clone().len(), 1);
            assert_eq!(auction.proposals.get(participants[0]), 
                Some((res.0, res.1, res.2,))
            );
        }

        #[ink::test]
        fn can_complete_auction() {
            let slot_ids = vec![vec![1,2,3], vec![2,3,4], vec![3,4,5]];
            let threshold = 2;
            // we'll pretend that the blockchain is seeded with these params
            let ibe_params = test_ibe_params();
            let seed_hash = crypto::utils::sha256(&crypto::utils::sha256(b"test1"));
            let rng = ChaCha20Rng::from_seed(seed_hash.try_into().expect("should be 32 bytes; qed"));
            // setup auction
            let mut auction = TlockAuction::new(b"test1".to_vec(), 1u32, 1u8, slot_ids.clone(), threshold);
            let res = add_bid(slot_ids.clone(), threshold, ibe_params.0.clone(), ibe_params.1, rng);
            auction.propose(res.0.clone(), res.1.clone(), res.2.clone());
            // prepare IBE slot secrets
            // in practice this would be fetched from block headers
            let ibe_slot_secrets: Vec<Vec<u8>> = ibe_extract(ibe_params.2, slot_ids).into_iter()
                .map(|(sk, _)| sk).collect::<Vec<_>>();
            // complete the auction
            auction.complete(ibe_params.0, ibe_slot_secrets);

            let revealed_bids = auction.revealed_bids;
            assert_eq!(revealed_bids.len(), 1);
            assert_eq!(revealed_bids[0], b"{I want to bid X tokens for your NFT}".to_vec());
        }

        fn add_bid(
                slot_ids: Vec<Vec<u8>>,
                threshold: u8,
                p: Vec<u8>, q: Vec<u8>, 
                rng: ChaCha20Rng
            ) -> (Vec<u8>, Vec<u8>, Vec<Vec<u8>>) {
            let mock_bid_tx = b"{I want to bid X tokens for your NFT}".to_vec();
            let res = 
                DefaultEtfClient::<BfIbe>::encrypt(
                    p, q, &mock_bid_tx, slot_ids, threshold, rng
                ).unwrap();
            (
                res.aes_ct.ciphertext.clone(),
                res.aes_ct.nonce.clone(),
                res.etf_ct.clone(),
            )
        }
    }


    // /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    // ///
    // /// When running these you need to make sure that you:
    // /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    // /// - Are running a Substrate node which contains `pallet-contracts` in the background
    // #[cfg(all(test, feature = "e2e-tests"))]
    // mod e2e_tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;

    //     /// A helper function used for calling contract messages.
    //     use ink_e2e::build_message;

    //     /// The End-to-End test `Result` type.
    //     type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    //     /// We test that we can upload and instantiate the contract using its default constructor.
    //     #[ink_e2e::test]
    //     async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = SealedBidAuctionRef::default();

    //         // When
    //         let contract_account_id = client
    //             .instantiate("sealed_bid_auction", &ink_e2e::alice(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         // Then
    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         Ok(())
    //     }

    //     /// We test that we can read and write a value from the on-chain contract contract.
    //     #[ink_e2e::test]
    //     async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = SealedBidAuctionRef::new(false);
    //         let contract_account_id = client
    //             .instantiate("sealed_bid_auction", &ink_e2e::bob(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         // When
    //         let flip = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.flip());
    //         let _flip_result = client
    //             .call(&ink_e2e::bob(), flip, 0, None)
    //             .await
    //             .expect("flip failed");

    //         // Then
    //         let get = build_message::<SealedBidAuctionRef>(contract_account_id.clone())
    //             .call(|sealed_bid_auction| sealed_bid_auction.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), true));

    //         Ok(())
    //     }
    // }
}
