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
use std::collections::HashMap;


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
    token_counter: TokenCounter
    
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
    pub fn new(owner_id: AccountId) -> Self {
        // require!(!env::state_exists(), "Already initialized");
    
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
            token_counter: TokenCounter::new()
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
        let title = format!("Unit {split_identifier} of property {property_identifier}");
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
        let token_id = self.token_counter.increment().to_string();
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id, "Unauthorized");
        self.tokens.internal_mint(token_id, token_owner_id.clone(), Some(token_metadata))
    }

    #[payable]
    pub fn multi_nft_mint(
        &mut self,
        token_owner_id: &AccountId,
        property_identifier: String,
        doc_urls: String,
        image_url: String
    ) -> Vec<(Token, String)> {
        assert_eq!(env::predecessor_account_id(), self.tokens.owner_id, "Unauthorized");

        let mut split_id = 1;
        
        let doc_splits = doc_urls.split(",");
        let mut tokens = Vec::with_capacity(doc_splits.clone().collect::<Vec::<_>>().len());

        for  doc in doc_splits {

            let id_length = &split_id.to_string().chars().count();
            let zero_spacing = "0".repeat(4 - id_length);

            let split_identifier = format!("{}{}{}", property_identifier.to_string(), zero_spacing, &split_id.to_string());

            let title = format!("Unit {split_id} of property {property_identifier}");
            let description = format!("Token for split {split_id} of property {property_identifier}");
            let token_metadata = TokenMetadata {
                title: Some(title),
                description: Some(description),
                media: Some(image_url.clone()),
                media_hash: Some(Base64VecU8::from(image_url.clone().as_bytes().to_vec())),
                copies: Some(1u64),
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: Some(doc.clone().to_string()),
                reference_hash: Some(Base64VecU8::from(doc.as_bytes().to_vec())),
            };

            let token_id = self.token_counter.increment().to_string();
            let token = self.tokens.internal_mint(token_id, token_owner_id.clone(), Some(token_metadata));

            tokens.push((token, split_identifier));

            split_id += 1;
        }

        tokens

    }

    pub fn get_user_properties(&self, account_id: AccountId) -> Vec<(TokenId, TokenMetadata)> {

        // let tokens = &mut self.tokens.tokens_per_owner;

        let mut user_tokens = Vec::new();
        let mut return_value = Vec::new();

        if let Some(tokens_per_owner) = &self.tokens.tokens_per_owner {
            user_tokens = tokens_per_owner.get(&account_id).unwrap_or(UnorderedSet::new(b"s")).to_vec();
        }

        if let Some(token_metadata_by_id) = &self.tokens.token_metadata_by_id {

            return_value = user_tokens.into_iter().map(|token_id| {
                let metadata = token_metadata_by_id.get(&token_id).unwrap();

                (token_id, metadata)
            }).collect();
            
        }

        return_value
        
    }


    pub fn set_contract_owner(&mut self, account_id: AccountId) {
        self.tokens.owner_id = account_id;
    }

    pub fn get_token(&self, token_id: TokenId) -> Token {

        self.nft_token(token_id).unwrap_or_else(|| env::panic_str("Token with provided ID doesn't exist"))
    }

    pub fn nft_token(&self, token_id: TokenId) -> Option<Token> {

        let owner_id = self.tokens.owner_by_id.get(&token_id)?;

        let mut metadata = None;
        let mut approved_account_ids = None;

        if let Some(meta) = &self.tokens.token_metadata_by_id {
            metadata = meta.get(&token_id);
        } 

        if let Some(appr) = &self.tokens.approvals_by_id {
            approved_account_ids = appr.get(&token_id);
        }

        Some(Token { token_id, owner_id, metadata, approved_account_ids })

    }

    pub fn approve_token_spender(&mut self, spender: AccountId, token_id: TokenId) {
        let owner = self.tokens.owner_by_id.get(&token_id).unwrap();

        require!(owner == env::signer_account_id(), "NFT Approve: Unauthorized");

        if let Some(next_approval_id_by_id) = &mut self.tokens.next_approval_id_by_id {

            let approval_id = next_approval_id_by_id.get(&token_id).unwrap_or(1);

            if let Some(approval_by_id) = &mut self.tokens.approvals_by_id {

                let mut app_id = approval_by_id.get(&token_id).unwrap_or(HashMap::new());

                app_id.insert(spender.clone(), approval_id.clone());

                approval_by_id.insert(&token_id, &app_id);
            }

            let next_approval_id = approval_id + 1;

            next_approval_id_by_id.insert(&token_id, &next_approval_id);

        }
        
    }

    pub fn transfer_token(&mut self, token_id: TokenId, receiver: AccountId) -> AccountId {
        require!(self.tokens.owner_id == env::predecessor_account_id(), "Unauthorized");
        let mut approval_id = None;
        let sender = env::predecessor_account_id();

        if let Some(approval_by_id) = &mut self.tokens.approvals_by_id {

            let app_id = approval_by_id.get(&token_id).unwrap_or(HashMap::new());

            approval_id = Some(app_id[&sender]);
        }

        let previous_owner = self.tokens.owner_by_id.get(&token_id).unwrap();

        let memo = format!("Transfer for sale of property split with token ID {} from {} to {}", &token_id, previous_owner, &receiver);

        self.tokens.internal_transfer(&sender, &receiver, &token_id, approval_id.clone(), Some(memo));

        self.tokens.owner_by_id.get(&token_id).unwrap()
    }
}

//near_contract_standards::impl_non_fungible_token_core!(RietsToken, tokens);
near_contract_standards::impl_non_fungible_token_approval!(RietsToken, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(RietsToken, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for RietsToken {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

