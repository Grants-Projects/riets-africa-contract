
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, Gas, Promise, ext_contract, require};
use near_contract_standards::non_fungible_token::{Token, TokenId, metadata::TokenMetadata};
use std::convert::From;

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
    pub fn new(id_: U128, name_: String, splits_: u8, property_identifier_: String, valuation_: Balance, image_: String) -> Self {
        Self {
            id: id_,
            name: name_,
            image: image_,
            property_identifier: property_identifier_,
            valuation: valuation_,
            split_ids: Vec::with_capacity(splits_ as usize)
        }
    }

    pub fn set_valuation(&mut self, value: u128) -> bool {
        self.valuation = value;
        true
    }
}

pub struct PropertyWithSplits<'a> {
    id: U128,
    name: String,
    image: String,
    property_identifier: String,
    valuation: Balance,
    property_splits: Vec<&'a PropertySplit>,
}

pub struct PropertyWithSplit {
    id: U128,
    name: String,
    image: String,
    property_identifier: String,
    valuation: Balance,
    property_splits: Vec<PropertySplit>,
}

impl From<Property> for PropertyWithSplit {
    fn from(property: Property) -> Self {
        Self {
            id: property.id,
            name: property.name,
            image: property.image,
            property_identifier: property.property_identifier,
            valuation: property.valuation,
            property_splits: Vec::new()
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PropertySplit {
    id: U128,
    split_identifier: String,
    property_id: U128,
    token_id: TokenId,
    token_metadata: TokenMetadata,
    owner: AccountId,
    last_sale_date: u64,
    on_sale: bool
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PurchaseOffer {
    id: U128,
    value: Balance,
    buyer: AccountId,
    token_id: TokenId
}



#[ext_contract(ext_nft_contract)]
trait RietsToken {
    fn nft_mint(
        &self,
        token_owner_id: AccountId,
        property_identifier: String,
        split_identifier: String,
        doc_url: String,
        image_url: String
    ) -> Token;

    fn get_user_properties(
        &self,
        account_id: &AccountId
    ) -> Vec<TokenId>;

    fn transfer_token(
        &self,
        token_id: TokenId, 
        receiver: AccountId
    );
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct RietsAfrica {
    properties: Vec<Property>,
    property_splits: Vec<PropertySplit>,
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
      

        let new_property_id = U128::from(u128::from(self.properties.len() as u64) + 1);

        let property = Property::new(
            new_property_id.clone(), 
            name, 
            doc_urls.len() as u8,
            identifier.clone(), 
            valuation, 
            image_url.clone()
        );
        self.properties.push(property);

        let mut split_id = 1;

        for  doc in doc_urls {

            let id_length = &split_id.to_string().chars().count();
            let property_identifier = identifier.clone();
            let zero_spacing = "0".repeat(4 - id_length);

            let split_identifier = format!("{}{}{}", property_identifier.to_string(), zero_spacing, &split_id.to_string());

            split_id += 1;

            ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .nft_mint(
                self.owner.clone(),
                identifier.clone(),
                split_identifier.clone(),
                doc,
                image_url.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_mint_nft_callback(new_property_id.clone(), split_identifier) 
            );

            
        }
    }

    pub fn set_property_valuation(&mut self, property_id: U128, new_valuation: Balance) {
        require!(env::signer_account_id() == self.owner, "Not authorised");
        let id = property_id.clone();
        self.properties[(id.0 - 1) as usize].set_valuation(new_valuation);

        // property.set_valuation(new_valuation);
        // self.properties[u128::from(&property_id)] = property;
    }

    #[payable]
    pub fn make_property_offer(&mut self, property_split_id: U128) {
        
        let choice_split = &self.property_splits[(property_split_id.clone().0 as usize) - 1];

        let buyer = env::signer_account_id();

        require!(buyer != self.owner && buyer != choice_split.owner.clone(), "Not authorized");

        require!(env::attached_deposit() >= self.get_split_value(&property_split_id), "Not sufficient deposit to make offer");

        let mut previous_offers_on_split = self.offers.get(&property_split_id).unwrap_or(Vec::new());

        let offer_id = U128::from(u128::from((previous_offers_on_split.len() as u64) + 1));

        let offer = PurchaseOffer {
            id: offer_id,
            value: env::attached_deposit(),
            buyer: env::signer_account_id(),
            token_id: choice_split.token_id.clone()
        };

        previous_offers_on_split.push(offer);

        self.offers.insert(&property_split_id, &previous_offers_on_split);

    }


    pub fn sell_property_to_offer(&mut self, property_split_id: U128, offer_id: U128) {

        let property_split = &self.property_splits[(property_split_id.clone().0 as usize) - 1];

        require!(if property_split.last_sale_date == 0 {
            env::signer_account_id() == self.owner
        } else {
            env::signer_account_id() == property_split.owner
        }, "Not authorised to sell this property");

        let offers_on_split = self.offers.get(&property_split_id).unwrap_or_else(|| env::panic_str("No offer on this property"));

        let offer = &offers_on_split[(offer_id.clone().0 as usize) - 1];

        ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .transfer_token(
                offer.token_id.clone(),
                offer.buyer.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_transfer_token_callback_on_sale(property_split_id, &property_split.token_id, offer.buyer.clone()) 
            );

    }

    pub fn place_property_split_on_sale(&mut self, property_split_id: U128) {

        let property_split = &self.property_splits[(property_split_id.clone().0 as usize) - 1];
        let token_id = &property_split.token_id;

        let mut split = self.property_split_by_token_id.get(token_id).unwrap();

        require!(if property_split.last_sale_date == 0 {
            env::signer_account_id() == self.owner
        } else {
            env::signer_account_id() == property_split.owner
        }, "Not authorised to sell this property");

        split.on_sale = true;

        self.property_split_by_token_id.insert(token_id, &split);

        self.property_splits[(property_split_id.clone().0 as usize) - 1] = split;

    }

    pub fn buy_from_sale(&mut self, property_split_id: U128) {

        let property_split = &self.property_splits[(property_split_id.clone().0 as usize) - 1];

        let buyer = env::signer_account_id();

        require!(buyer != self.owner && buyer != property_split.owner, "Not authorized");

        require!(property_split.on_sale, "Property is not available for sale");

        require!(env::attached_deposit() >= self.get_split_value(&property_split_id), "Not sufficient deposit to make offer");

        ext_nft_contract::ext(AccountId::new_unchecked(NFT_CONTRACT.to_string()))
            .transfer_token(
                property_split.token_id.clone(),
                buyer.clone())
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_transfer_token_callback_on_sale(property_split_id, &property_split.token_id, buyer) 
            );
    }


    // returns the value of a split based on the actual property valuation
    pub fn get_split_value(&self, property_split_id: &U128) -> Balance {

        let property_split = &self.property_splits[(property_split_id.clone().0 as usize) - 1];

        let property = &self.properties[(property_split.property_id.0 - 1) as usize];

        let value = property.valuation;
        let splits = u128::from(property.split_ids.len() as u64);

        let split_value = value*100/splits;

        split_value/100
    }


    pub fn get_properties(&self) -> Vec<PropertyWithSplits> {

        let properties = &self.properties;

        properties.into_iter().map(|property| {

            let ids = &property.split_ids;

            let splits = ids.into_iter().map(|split_id| &self.property_splits[(split_id.0 as usize) - 1]).collect::<Vec<&PropertySplit>>();
            //PropertyWithSplit::from(property)

            PropertyWithSplits {
                id: property.id.clone(),
                name: property.name.clone(),
                image: property.image.clone(),
                property_identifier: property.property_identifier.clone(),
                valuation: property.valuation.clone(),
                property_splits: splits
            }
        }).collect::<Vec<PropertyWithSplits>>()
    }

    // pub fn get_user_properties(&self, account_id: AccountId) -> Vec<PropertyWithSplits> {

    //     let mut properties = &self.properties;

    //     let prop_with_splits = properties.into_iter().map(|property| {

    //         let splits = &property.split_ids.into_iter().map(|split_id| &self.property_splits[(split_id.clone().0 as usize) - 1]).collect::<Vec<PropertySplit>>();

    //         let splits_of_owner = splits.into_iter().filter(|split| split.owner == account_id).collect::<Vec<&PropertySplit>>();

    //         PropertyWithSplits {
    //             id: property.id.clone(),
    //             name: property.name.clone(),
    //             image: property.image.clone(),
    //             property_identifier: property.property_identifier.clone(),
    //             valuation: property.valuation.clone(),
    //             property_splits: splits_of_owner
    //         }
    //     });

    //     prop_with_splits.filter(|property| property.property_splits.len() > 0).collect::<Vec<PropertyWithSplits>>()
    // }


    pub fn get_split_offers(&self, property_split_id: U128) -> Vec<PurchaseOffer> {

        self.offers.get(&property_split_id).unwrap_or(Vec::new())
    }

    pub fn get_splits_on_sale(&self) -> Vec<&PropertySplit> {
        let splits = &self.property_splits;
         splits.into_iter().filter(|split| split.on_sale).collect::<Vec<&PropertySplit>>()
    }


    #[private]
    pub fn on_mint_nft_callback(&mut self, property_id: U128, split_identifier: String, #[callback_unwrap] token: Token) {
        let splits_count = self.property_splits.len() as u64;
        let split_id = U128::from(u128::from(splits_count) + 1);

        let token_minted = token.clone();
        
        let property_split = PropertySplit {
            id: split_id.clone(),
            split_identifier: split_identifier,
            property_id: property_id.clone(),
            token_id: token_minted.token_id,
            token_metadata: token_minted.metadata.unwrap(),
            owner: env::signer_account_id(),
            last_sale_date: 0,
            on_sale: false
        };

        self.property_split_by_token_id.insert(&token.token_id, &property_split);

        self.properties[(property_id.0 - 1) as usize].split_ids.push(split_id);

        self.property_splits.push(property_split);

    }

    #[private]
    pub fn on_transfer_token_callback_on_sale(&mut self, property_split_id: U128, property_token_id: &TokenId, buyer: AccountId) {

        let property_split = &self.property_splits[(property_split_id.0 - 1) as usize];
        let token_id = &property_split.token_id;
        let mut split = self.property_split_by_token_id.get(token_id).unwrap();

        split.owner = buyer;
        split.last_sale_date = env::block_timestamp_ms();
        split.on_sale = false;
        
        self.property_split_by_token_id.insert(property_token_id, &split);

        self.offers.insert(&property_split.id, &Vec::new());

        self.property_splits[(property_split_id.0 - 1) as usize] = split;
    }

    
}

