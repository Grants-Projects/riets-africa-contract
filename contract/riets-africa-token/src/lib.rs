/*!
Non-Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use near_sdk::json_types::Base64VecU8;
use iso8601::datetime;


#[derive(Default, BorshDeserialize, BorshSerialize)]
pub struct TokenCounter {
    value: u128
}

impl TokenCounter {
    pub fn new() -> Self {
        Self {
            value: 0
        }
    }

    pub fn increment(&mut self) -> u128 {
        self.value = self.value + 1;
        self.value
    }
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct RietsToken {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    tokenCounter: TokenCounter,
    
}

// const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl RietsToken {

    #[init]
    pub fn new() -> Self {
        // require!(!env::state_exists(), "Already initialized");
        let owner_id = AccountId::new_unchecked(String::from("riets-africa.testnet"));
        let metadata = NFTContractMetadata {
                        spec: NFT_METADATA_SPEC.to_string(),
                        name: "Riets Africa Property Token".to_string(),
                        symbol: "RIET-A".to_string(),
                        icon: None,
                        base_uri: None,
                        reference: None,
                        reference_hash: None,
                    };
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            tokenCounter: TokenCounter::new()
        }
    }

    /// Mint a new token with ID=`token_id` belonging to `token_owner_id`.
    ///
    /// Since this example implements metadata, it also requires per-token metadata to be provided
    /// in this call. `self.tokens.mint` will also require it to be Some, since
    /// `StorageKey::TokenMetadata` was provided at initialization.
    ///
    /// `self.tokens.mint` will enforce `predecessor_account_id` to equal the `owner_id` given in
    /// initialization call to `new`.
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_owner_id: &AccountId,
        property_identifier: String,
        split_identifier: String,
        doc_url: String,
        image_url: String,
    ) -> Token {
        let title = format!("Property {property_identifier} Unit {split_identifier}");
        let description = format!("Token for split {split_identifier} of property {property_identifier}");
        let token_metadata = TokenMetadata {
            title: Some(title),
            description: Some(description),
            media: Some(image_url.clone()),
            media_hash: Some(Base64VecU8::from(image_url.as_bytes().to_vec())),
            copies: Some(1u64),
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: Some(doc_url.clone()),
            reference_hash: Some(Base64VecU8::from(doc_url.as_bytes().to_vec())),
        };
        let token_id = self.tokenCounter.increment().to_string();
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id, "Unauthorized");
        self.tokens.internal_mint(token_id, token_owner_id.clone(), Some(token_metadata))
    }

    pub fn get_user_properties(&self, account_id: AccountId) -> Vec<TokenMetadata> {

        let tokens = &self.tokens.tokens_per_owner;

        let t = &tokens.clone();

        let user_tokens = t.unwrap_or(LookupMap::new(b"m")).get(&account_id).unwrap_or(UnorderedSet::new(b"s"));

        // for token in user_tokens.iter() {

        // }

        Vec::new()
    }
}

near_contract_standards::impl_non_fungible_token_core!(RietsToken, tokens);
near_contract_standards::impl_non_fungible_token_approval!(RietsToken, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(RietsToken, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for RietsToken {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

// #[cfg(all(test, not(target_arch = "wasm32")))]
// mod tests {
//     use near_sdk::test_utils::{accounts, VMContextBuilder};
//     use near_sdk::testing_env;
//     use std::collections::HashMap;

//     use super::*;

//     const MINT_STORAGE_COST: u128 = 5870000000000000000000;

//     fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
//         let mut builder = VMContextBuilder::new();
//         builder
//             .current_account_id(accounts(0))
//             .signer_account_id(predecessor_account_id.clone())
//             .predecessor_account_id(predecessor_account_id);
//         builder
//     }

//     fn sample_token_metadata() -> TokenMetadata {
//         TokenMetadata {
//             title: Some("EA Kazi Certificate".into()),
//             description: Some("Certificate for completion of Blockchain course".into()),
//             media: None,
//             media_hash: None,
//             copies: Some(1u64),
//             issued_at: None,
//             expires_at: None,
//             starts_at: None,
//             updated_at: None,
//             extra: None,
//             reference: None,
//             reference_hash: None,
//         }
//     }

//     #[test]
//     fn test_new() {
//         let mut context = get_context(accounts(1));
//         testing_env!(context.build());
//         let contract = EAKaziCertificate::new();
//         testing_env!(context.is_view(true).build());
//         assert_eq!(contract.nft_token("1".to_string()), None);
//     }

//     #[test]
//     #[should_panic(expected = "The contract is not initialized")]
//     // fn test_default() {
//     //     let context = get_context(accounts(1));
//     //     testing_env!(context.build());
//     //     let _contract = EAKaziCertificate::default();
//     // }

//     #[test]
//     fn test_mint() {
//         let mut context = get_context(accounts(0));
//         testing_env!(context.build());
//         let mut contract = EAKaziCertificate::new_default_meta(accounts(0).into());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(MINT_STORAGE_COST)
//             .predecessor_account_id(accounts(0))
//             .build());

//         let token_id = "0".to_string();
//         let token = contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());
//         assert_eq!(token.token_id, token_id);
//         assert_eq!(token.owner_id, accounts(0));
//         assert_eq!(token.metadata.unwrap(), sample_token_metadata());
//         assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
//     }

//     #[test]
//     fn test_transfer() {
//         let mut context = get_context(accounts(0));
//         testing_env!(context.build());
//         let mut contract = Contract::new_default_meta(accounts(0).into());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(MINT_STORAGE_COST)
//             .predecessor_account_id(accounts(0))
//             .build());
//         let token_id = "0".to_string();
//         contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(1)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_transfer(accounts(1), token_id.clone(), None, None);

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .account_balance(env::account_balance())
//             .is_view(true)
//             .attached_deposit(0)
//             .build());
//         if let Some(token) = contract.nft_token(token_id.clone()) {
//             assert_eq!(token.token_id, token_id);
//             assert_eq!(token.owner_id, accounts(1));
//             assert_eq!(token.metadata.unwrap(), sample_token_metadata());
//             assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
//         } else {
//             panic!("token not correctly created, or not found by nft_token");
//         }
//     }

//     #[test]
//     fn test_approve() {
//         let mut context = get_context(accounts(0));
//         testing_env!(context.build());
//         let mut contract = Contract::new_default_meta(accounts(0).into());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(MINT_STORAGE_COST)
//             .predecessor_account_id(accounts(0))
//             .build());
//         let token_id = "0".to_string();
//         contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

//         // alice approves bob
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(150000000000000000000)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_approve(token_id.clone(), accounts(1), None);

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .account_balance(env::account_balance())
//             .is_view(true)
//             .attached_deposit(0)
//             .build());
//         assert!(contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
//     }

//     #[test]
//     fn test_revoke() {
//         let mut context = get_context(accounts(0));
//         testing_env!(context.build());
//         let mut contract = Contract::new_default_meta(accounts(0).into());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(MINT_STORAGE_COST)
//             .predecessor_account_id(accounts(0))
//             .build());
//         let token_id = "0".to_string();
//         contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

//         // alice approves bob
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(150000000000000000000)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_approve(token_id.clone(), accounts(1), None);

//         // alice revokes bob
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(1)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_revoke(token_id.clone(), accounts(1));
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .account_balance(env::account_balance())
//             .is_view(true)
//             .attached_deposit(0)
//             .build());
//         assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), None));
//     }

//     #[test]
//     fn test_revoke_all() {
//         let mut context = get_context(accounts(0));
//         testing_env!(context.build());
//         let mut contract = Contract::new_default_meta(accounts(0).into());

//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(MINT_STORAGE_COST)
//             .predecessor_account_id(accounts(0))
//             .build());
//         let token_id = "0".to_string();
//         contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

//         // alice approves bob
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(150000000000000000000)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_approve(token_id.clone(), accounts(1), None);

//         // alice revokes bob
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .attached_deposit(1)
//             .predecessor_account_id(accounts(0))
//             .build());
//         contract.nft_revoke_all(token_id.clone());
//         testing_env!(context
//             .storage_usage(env::storage_usage())
//             .account_balance(env::account_balance())
//             .is_view(true)
//             .attached_deposit(0)
//             .build());
//         assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
//     }
// }