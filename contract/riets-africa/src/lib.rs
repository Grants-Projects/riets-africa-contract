
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, LazyOption, LookupSet};
use near_sdk::json_types::U128;
use near_sdk::{env, log, near_bindgen, AccountId, Balance, Gas, Promise, ext_contract, require};
use near_contract_standards::non_fungible_token::{Token, TokenId};

pub const NFT_CONTRACT: &str = "certificate.eakazi.testnet";
pub const XCC_GAS: Gas = Gas(20000000000000);


#[derive(BorshDeserialize, BorshSerialize)]
pub struct Property {
    id: U128,
    name: String,
    property_identifier: String,
    valuation: Balance,
    property_splits: Vec<PropertySplit>
}


impl Property {
    pub fn new(id_: U128, name_: String, property_identifier_: String, valuation_: Balance, splits_: u64) -> Self {
        Self {
            id: id_,
            name: name_,
            property_identifier: property_identifier_,
            valuation: valuation_,
            property_splits: Vec::with_capacity(splits_)
        }
    }

    pub fn set_valuation(mut self, value: U128) -> bool {
        self.valuation = value;
        true
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PropertySplit {
    id: U128,
    split_identifier: String,
    token_id: TokenId,
    owner: AccountId,
    last_sale_date: u64
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PurchaseOffer {
    id: U128,
    value: Balance,
    buyer: AccountId
    token_id: TokenId,
    owner: AccountId
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

    fn get_user_properties
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct RietsAfrica {
    properties: Vec<Property>,
    owner: AccountId,
    offers: LookupMap<TokenId, Vec<PurchaseOffer>
}

impl Default for RietsAfrica {
    fn default() -> Self {
        Self {
            properties: Vec::new(),
            owner: env::signer_account_id();
        }
    }
}


#[near_bindgen]
impl RietsAfrica {

    pub fn create_property(&mut self, name: String, image_url: String, identifier: String, splits: u64, valuation: Balance, doc_urls: Vec<String>) {
        require!(doc_urls.len() == splits, "JSON length supplied do not match number of splits");

        let new_property_id = self.properties.len();

        let mut property = Property::new(U128::from(&new_property_id), name, identifier.clone(), valuation, splits);
        self.properties.push(property);

        let mut split_id = 0;

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
                    .on_mint_nft_callback(split_id - 1, &split_identifier, &new_property_id) 
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
    pub fn make_property_offer(&mut self, property_id: U128, new_valuation: U128) {
        let mut property = self.properties[u128::from(&property_id)];
        require!(env::attached_deposit() >= property.valuation, "Not sufficient deposit");


    }

    pub fn get_user_properties(account_id: String) {


    }


    
    pub fn bid_property_unit(&mut self, course_id: U128) {
        let trainee = env::signer_account_id();
        let mut registered_trainees = self.enrolments.get(&course_id).unwrap_or_default();
        registered_trainees.push(trainee);
        self.enrolments.insert(&course_id, &registered_trainees);
    }

    pub fn mint_certificate_to_trainee(&mut self, course_id: U128, trainee_id: AccountId, certificate_url: String, issue_date: String) {
        let course_ = self.get_course_by_id(&course_id).unwrap();

        // require!(course_  false, "Course with requested id does not exist");
        ext_certificate_contract::ext(AccountId::new_unchecked(CERTIFICATE_CONTRACT.to_string()))
            .nft_mint(
                trainee_id.clone(),
                self.course_trainer.get(&course_id).unwrap(),
                course_.name.clone(),
                certificate_url,
                issue_date)
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .on_mint_certificate_callback(trainee_id, course_.skills.clone(), U128::from(course_.id), self.course_trainer.get(&course_id).unwrap())
            );

        
    }

    // for jobs exceeding 12 months, 12 month wage would be initially deducted
    #[payable]
    pub fn create_job(&mut self, job_id_: U128, name: String, description_: String, skills_: Vec<U128>, wage: U128, number_of_roles: u128, duration: u128) {

        let wage_to_pay = wage.0 * number_of_roles * duration;
        require!(env::attached_deposit() >= wage_to_pay, "Attach wage to be paid to escrow first");
        let job_owner = env::signer_account_id();
        let job = Job::new(job_id_, name, description_, skills_, job_owner.clone(), wage, number_of_roles);
        self.jobs.insert(&job_id_, &job);
        let previous_wages_in_escrow = self.wages_in_escrow.get(&job_owner).unwrap_or(U128::from(0));
        let increment_wage = U128::from(wage_to_pay + previous_wages_in_escrow.0);
        self.wages_in_escrow.insert(&job_owner, &increment_wage);
    }

    pub fn apply_to_job(&mut self, job_id: U128) {
        require!(self.job_exists(&job_id), "Job with provided id does not exist");

        let applicant = env::signer_account_id();
        let trainers_for_job = self.user_has_skills_for_job(&applicant, &job_id).unwrap_or_default();
        
        require!(trainers_for_job.len() > 0, "You are not skilled enough for the job");

        let mut trainers = Vec::new();

        for trainer in trainers_for_job {
            trainers.push(trainer.clone());
        }

        let application = JobApplication::new(job_id.clone(), trainers, applicant.clone());
        let mut apps = self.job_applications.get(&job_id).unwrap();
        apps.insert(&applicant, &application);

        self.job_applications.insert(&job_id, &apps);

    }

    pub fn confirm_job_emloyment(&mut self, job_id: U128, applicant: AccountId) {
        require!(self.job_exists(&job_id), "Job with provided id does not exist");
        require!(env::signer_account_id() == self.get_job_by_id(&job_id).unwrap().job_owner, "Only job owner can confirm employment");

        let mut apps = self.job_applications.get(&job_id).unwrap();
        require!(&apps.contains_key(&applicant), "The applicant has not applied for this job");
        
        let mut job_app = apps.get(&applicant).unwrap();

        job_app.status = 1;
        job_app.start_date = Some(env::block_timestamp());
        
        apps.insert(&applicant, &job_app);
        self.job_applications.insert(&job_id, &apps);
    }


    pub fn end_job_employment(&mut self, job_id: U128, applicant: AccountId) {
        require!(self.job_exists(&job_id), "Job with provided id does not exist");
        require!(env::signer_account_id() == self.get_job_by_id(&job_id).unwrap().job_owner, "Only job owner can confirm employment");

        let mut apps = self.job_applications.get(&job_id).unwrap();
        require!(&apps.contains_key(&applicant), "The applicant has not applied for this job");
        
        let mut job_app = apps.get(&applicant).unwrap();

        job_app.status = 2;
        
        apps.insert(&applicant, &job_app);
        self.job_applications.insert(&job_id, &apps);
    }

    pub fn pay_wage(&mut self, job_id: U128, applicant: AccountId) {
        require!(self.job_exists(&job_id), "Job with provided id does not exist");
        let job = self.get_job_by_id(&job_id).unwrap();
        let owner = job.job_owner;
        require!(env::signer_account_id() == owner.clone(), "Only job owner can confirm wage payment");

        let mut wage_in_excrow = self.wages_in_escrow.get(&owner).unwrap_or(U128::from(0));
        let wage_for_job = job.wage;

        let application = self.get_job_aplication(&job_id, &applicant).unwrap();
        let trainers = application.trainers_to_pay;

        let wage_to_applicant = (wage_for_job.0 * 9)/10;
        let wage_to_trainer = (wage_for_job.0 - wage_to_applicant)/u128::try_from(trainers.len()).unwrap_or(1);

        wage_in_excrow = U128::from(wage_in_excrow.0 - wage_for_job.0);

        Promise::new(applicant).transfer(wage_to_applicant)
            .then(Self::ext(env::current_account_id())
                .with_static_gas(XCC_GAS)
                .on_pay_wage_callback(owner, wage_in_excrow));

        for trainer in trainers {
            Promise::new(trainer).transfer(wage_to_trainer);
        }
    }



    // will check if user has the skills for the job and return the trainers who taught the skill
    #[private]
    pub fn user_has_skills_for_job(&self, user: &AccountId, job_id: &U128) -> Option<Vec<AccountId>> {
        let job = self.get_job_by_id(job_id).unwrap();
        let job_skills = &job.skills;
        let user_certs = self.certificates.get(&user).unwrap_or_default();

        // number of skills user has for job
        let mut number_of_matches = 0;

        // trainers that would be paid from the job
        let mut trainers_for_skills = Vec::new();

        for skill in job_skills.clone() {
            let mut current_skill_check = 0;

            for cert in &user_certs {
                if cert.skills.contains(&skill) {
                    trainers_for_skills.push(&cert.issuer);
                    if current_skill_check > 1 {
                        continue
                    }
                    number_of_matches = number_of_matches + 1;
                    current_skill_check = current_skill_check + 1;
                }
            }
        }

        let mut trainers = Vec::new();
        for trainer in trainers_for_skills {
            trainers.push(trainer.clone());
        }

        let percentage_match = number_of_matches * 100/u128::try_from(job_skills.len()).unwrap_or(1);
        if percentage_match > 50 {
            return Some(trainers);
        }

        None
    }

    #[private]
    pub fn course_exists(&self, course_id_: &U128) -> bool {
        self.courses.contains_key(course_id_)
    }

    #[private]
    pub fn job_exists(&self, job_id: &U128) -> bool {
        self.jobs.contains_key(job_id)
    }

    #[private]
    pub fn is_user_enrolled(&self, course_id_: &U128, user_id: &AccountId) -> bool {
        for trainee in self.enrolments.get(course_id_).unwrap() {
            if &trainee == user_id {
                return true;
            }
        }
        false
    }

    #[private]
    #[result_serializer(borsh)]
    pub fn get_course_by_id(&self, course_id_: &U128) -> Option<Course> {
        self.courses.get(&course_id_)
    }

    #[private]
    #[result_serializer(borsh)]
    pub fn get_job_by_id(&self, job_id_: &U128) -> Option<Job> {
        self.jobs.get(&job_id_)
    }

    #[private]
    #[result_serializer(borsh)]
    pub fn get_job_aplication(&self, job_id_: &U128, applicant: &AccountId) -> Option<JobApplication> {
        self.job_applications.get(job_id_).unwrap().get(applicant)
    }

    #[private]
    pub fn on_mint_nft_callback(&mut self, split_id: &U128, split_identifier: &String, property_id: &u64, #[callback_unwrap] token: Token) {
        let mut property_splits = self.properties[&property_id].property_splits;
        
        let property_split = PropertySplit {
            id: split_id.clone(),
            split_identifier: split_identifier,
            token_id: token.token_id,
            last_sale_date: 0
        }

        property_splits.push(property_split);

        self.properties[&property_id].property_splits = property_splits;
    }

    
}

