
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LazyOption, LookupSet};
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, Gas, Promise, ext_contract, require};
use near_contract_standards::non_fungible_token::{Token, TokenId, metadata::TokenMetadata};

pub const NFT_CONTRACT: &str = "certificate.eakazi.testnet";
pub const XCC_GAS: Gas = Gas(20000000000000);


#[derive(BorshDeserialize, BorshSerialize)]
pub struct Property {
    id: U128,
    name: String,
    image: String,
    property_identifier: String,
    valuation: Balance,
    split_ids: Vec<U128>
}


impl Property {
    pub fn new(id_: U128, name_: String, splits_: u64, property_identifier_: String, valuation_: Balance, image_: String) -> Self {
        Self {
            id: id_,
            name: name_,
            image: image_,
            property_identifier: property_identifier_,
            valuation: valuation_,
            split_ids: Vec::with_capacity(splits_)
        }
    }

    pub fn set_valuation(&mut self, value: U128) -> bool {
        self.valuation = value;
        true
    }
}

pub struct PropertyWithSplits {
    id: &U128,
    name: &String,
    image: &String,
    property_identifier: &String,
    valuation: &Balance,
    property_splits: Vec<PropertySplit>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PropertySplit {
    id: U128,
    split_identifier: String,
    property_id: U128,
    token_id: TokenId,
    token_metadata: TokenMetadata,
    owner: AccountId,
    last_sale_date: u64
    on_sale: bool
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PurchaseOffer {
    id: U128,
    value: Balance,
    buyer: AccountId
    token_id: TokenId
}



#[ext_contract(ext_nft_contract)]
trait RietsToken {
    fn nft_mint(
        &self,
        token_owner_id: &AccountId,
        property_identifier: String,
        split_identifier: String,
        doc_url: String,
        image_url: String
    ) -> Token;

    fn get_user_properties(
        &self,
        account_id: &AccountId
    ) -> Vec<TokenId>

    fn get_user_properties(
        &self,
        account_id: &AccountId
    ) -> Vec<TokenId>

    fn transfer_token(
        &self,
        token_id: TokenId, 
        receiver: AccountId
    )
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct RietsAfrica {
    properties: Vec<Property>,
    property_splits: Vec<PropertySplit>
    owner: AccountId,
    property_split_by_token_id: LookupMap<TokenId, PropertySplit>,
    offers: LookupMap<U128, Vec<PurchaseOffer>>
}

impl Default for RietsAfrica {
    fn default() -> Self {
        Self {
            properties: Vec::new(),
            property_splits: Vec::new(),
            owner: env::signer_account_id(),
            property_split_by_token_id: LookupMap::new(b"p"),
            offers: LookupMap::new(b"o")
        }
    }
}


#[near_bindgen]
impl RietsAfrica {

    pub fn create_property(&mut self, name: String, image_url: String, identifier: String, valuation: Balance, doc_urls: Vec<String>) {
        require!(doc_urls.len() == splits, "JSON length supplied do not match number of splits");

        let new_property_id = self.properties.len();

        let mut property = Property::new(U128::from(u128::from(&new_property_id) + 1), name, identifier.clone(), valuation, image_url);
        self.properties.push(property);

        let mut split_id = 1;

        for let doc in doc_urls {

            let id_length = &split_id.to_string().chars().count();
            let property_identifier = property_splits.property_identifier;
            let zero_spacing = "0".repeat(4 - id_length);

            let split_identifier = format!({}{}{}, property_identifier.to_string(), zero_spacing, &split_id.to_string());

            split_id += 1;

            ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .nft_mint(
                self.owner,
                identifier,
                split_identifier.clone(),
                doc,
                image_url)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_mint_nft_callback(U128::from(u128::from(&new_property_id) + 1), &split_identifier, &new_property_id) 
            );

            
        }
    }

    pub fn set_property_valuation(&mut self, property_id: U128, new_valuation: U128) {
        require!(env::signer_account_id() == self.owner, "Not authorised");
        let mut property = self.properties[u128::from(&property_id)];

        property.set_valuation(new_valuation);
        self.properties[u128::from(&property_id)] = property;
    }

    #[payable]
    pub fn make_property_offer(&mut self, property_split_id: U128) -> PurchaseOffer {
        
        let choice_split = self.property_splits[&property_split_id.0 - 1];

        let buyer = env::signer_account_id();

        require!(&buyer != self.owner && &buyer != &choice_split.owner, "Not authorized");

        require!(env::attached_deposit() >= get_split_value(&property_split_id), "Not sufficient deposit to make offer");

        let mut previous_offers_on_split = self.offers.get(&property_split_id).unwrap_or(Vec::new());

        let offer_id = U128::from(&previous_offers_on_split.len() + 1);

        let offer = PurchaseOffer {
            id: offer_id,
            value: env::attached_deposit(),
            buyer: env::signer_account_id(),
            token_id: choice_split.token_id
        };

        previous_offers_on_split.push(&offer);

        self.offers.insert(&property_split_id, &previous_offers_on_split);

        offer

    }


    pub fn sell_property_to_offer(&mut self, property_split_id: U128, offer_id: U128) {

        let mut property_split = self.property_splits[&property_split_id.0 - 1];

        require!(if &property_split.last_sale_date == 0 {
            env::signer_account_id() == self.owner
        } else {
            env::signer_account_id() == &property_split.owner
        }, "Not authorised to sell this property");

        let offers_on_split = self.offers.get(&property_split_id).unwrap_or_else(|| env::panic_str("No offer on this property"));

        let offer = offers_on_split[&offer_id.0 - 1];

        ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .transfer_token(
                offer.token_id,
                offer.buyer)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_transfer_token_callback_on_sale(&property_split_id, &property_split, &offer) 
            );

    }

    pub fn place_property_split_on_sale(&mut self, property_split_id: U128) -> PropertySplit {

        let mut property_split = self.property_splits[&property_split_id.0 - 1];
        let token_id = &property_split.token_id;

        require!(if &property_split.last_sale_date == 0 {
            env::signer_account_id() == self.owner
        } else {
            env::signer_account_id() == &property_split.owner
        }, "Not authorised to sell this property");

        property_split.on_sale = true;

        self.property_split_by_token_id.insert(&token_id, &property_split)

        self.property_splits[&property_split_id.0 - 1] = property_split;

        property_split
    }

    pub fn buy_from_sale(&mut self, property_split_id: U128) {

        let mut property_split = self.property_splits[&property_split_id.0 - 1];

        let buyer = env::signer_account_id();

        require!(&buyer != self.owner && &buyer != &property_split.owner, "Not authorized");

        require!(&property_split.on_sale, "Property is not available for sale");

        require!(env::attached_deposit() >= get_split_value(&property_split_id), "Not sufficient deposit to make offer");

        ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .transfer_token(
                offer.token_id,
                offer.buyer)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_transfer_token_callback_on_sale(&property_split_id, &property_split, &buyer) 
            );
    }


    // returns the value of a split based on the actual property valuation
    pub fn get_split_value(&self, property_split_id: &U128) -> Balance {

        let mut property_split = self.property_splits[property_split_id.0 - 1];

        let property = self.properties[&property_split.property_id.0 - 1];

        let value = &property.valuation.0;
        let splits = u128::from(property.split_ids.len());

        let split_value = value*100/splits;

        split_value/100
    }


    pub fn get_properties(&self) -> Vec<PropertyWithSplits> {

        let mut properties = self.properties;

        properties.into_iter().map(|property| {

            let splits = &property.split_ids.into_iter().map(|split_id| self.property_splits[split_id.0 - 1]).collect::Vec<PropertySplit>();

            PropertyWithSplits {
                id: &property.id,
                name: &property.name,
                image: &property.image,
                property_identifier: &property.property_identifier,
                valuation: &property.valuation,
                property_splits: splits
            }
        }).collect()
    }

    pub fn get_user_properties(&self, account_id: AccountId) -> Vec<PropertyWithSplits> {

        let mut properties = self.properties;

        let prop_with_splits = properties.into_iter().map(|property| {

            let splits = &property.split_ids.into_iter().map(|split_id| self.property_splits[split_id.0 - 1]).collect::Vec<PropertySplit>();

            let splits_of_owner = splits.into_iter().filter(|split| *split.owner == account_id).collect::Vec<PropertySplit>();

            PropertyWithSplits {
                id: &property.id,
                name: &property.name,
                image: &property.image,
                property_identifier: &property.property_identifier,
                valuation: &property.valuation,
                property_splits: splits_of_owner
            }
        }).collect::Vec<PropertyWithSplits>();

        prop_with_splits.filter(|property| *property.property_splits.len() > 0).collect::Vec<PropertyWithSplits>();
    }


    pub fn get_split_offers(&self, property_split_id) -> Vec<PurchaseOffer> {

        self.offers.get(&property_split_id).unwrap_or(Vec::new())
    }

    pub fn get_splits_on_sale(&self) -> Vec<PropertySplit> {

        self.property_splits.into_iter().filter(|split| *split.on_sale).collect::Vec<PropertySplit>()
    }


    #[private]
    pub fn on_mint_nft_callback(&mut self, property_id: &U128, split_identifier: &String, property_id: &u64, #[callback_unwrap] token: Token) {
        let splits_count = self.property_splits.len();
        let split_id = U128::from(u128::from(splits_count) + 1)

        let token_minted = token.clone();
        
        let property_split = PropertySplit {
            id: split_id.clone(),
            split_identifier: split_identifier,
            token_id: token_minted.token_id,
            token_metadata: token_minted.metadata.unwrap(),
            owner: env::signer_account_id(),
            last_sale_date: 0,
            on_sale: false
        }

        self.property_split_by_token_id.insert(&token.token_id, &property_split);

        self.properties[property_id.0].split_ids.push(split_id);

        self.property_splits.push(property_split);

    }

    #[private]
    pub fn on_transfer_token_callback_on_sale(&self, property_split_id: &U128, property_split: &PropertySplit, buyer: &AccountId) -> Vec<PropertySplit> {

        let token_id = property_split.token_id;

        let mut new_property_split = property_split;

        new_property_split.owner = buyer;
        new_property_split.last_sale_date = env::block_timestamp_ms()

        self.property_splits[property_split_id.0 - 1] = new_property_split;
        
        self.property_split_by_token_id.insert(&token_id, new_property_split);

        self.offers.insert(&property_split_id, &Vec::new());
    }

    
}

