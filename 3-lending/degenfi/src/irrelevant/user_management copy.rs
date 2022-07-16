use scrypto::prelude::*;

#[derive(NonFungibleData, Describe, Encode, Decode, TypeId)]
pub struct User {
    #[scrypto(mutable)]
    deposit_balance: HashMap<ResourceAddress, Decimal>,
    #[scrypto(mutable)]
    borrow_balance: HashMap<ResourceAddress, Decimal>,
}

blueprint! {
    struct UserManagement {
        // Vault that holds the authorization badge
        user_badge_vault: Vault,
        // Collects User Address
        user_address: ResourceAddress,
        user_data: HashMap<NonFungibleId, User>,
    }

    impl UserManagement {
        pub fn new() -> ComponentAddress {
            // Badge that will be stored in the component's vault to update user state.
            let lending_protocol_user_badge = ResourceBuilder::new_fungible()
            .divisibility(DIVISIBILITY_NONE)
            .metadata("user", "Lending Protocol User Badge")
            .initial_supply(1);

            // NFT description for user identification. 
            let user_address = ResourceBuilder::new_non_fungible()
            .metadata("user", "Lending Protocol User")
            .mintable(rule!(require(lending_protocol_user_badge.resource_address())), LOCKED)
            .burnable(rule!(require(lending_protocol_user_badge.resource_address())), LOCKED)
            .updateable_non_fungible_data(rule!(require(lending_protocol_user_badge.resource_address())), LOCKED)
            .no_initial_supply();
            
            return Self {
                user_badge_vault: Vault::with_bucket(lending_protocol_user_badge),
                user_address: user_address,
                user_data: HashMap::new(),
            }
            .instantiate()
            .globalize()
        }

        // Creates a new user for the lending protocol.
        // User is created to track the deposit balance, borrow balance, and the token of each.
        // Token is registered by extracting the resource address of the token they deposited.
        // Users are not given a badge. Badge is used by the protocol to update the state. Users are given an NFT to identify as a user.

        pub fn new_user(&mut self) -> Bucket {

            // Mint NFT to give to users as identification 
            let user = self.user_badge_vault.authorize(|| {
                let resource_manager = borrow_resource_manager!(self.user_address);
                resource_manager.mint_non_fungible(
                    // The User id
                    &NonFungibleId::random(),
                    // The User data
                    User {
                        borrow_balance: HashMap::new(),
                        deposit_balance: HashMap::new(),
                    },
                )
            });
            // Returns NFT to user
            return user
        }

        pub fn get_user(&self, user_auth: Proof) -> NonFungibleId {
            let user_id = user_auth.non_fungible::<User>().id();
            return user_id
        }

        pub fn check_lien(&self, user_auth: Proof, token_requested: ResourceAddress) {
            // Check if deposit withdrawal request has no lien
            let user_badge_data: User = user_auth.non_fungible().data();
            assert!(user_badge_data.borrow_balance.get(&token_requested).unwrap() > &Decimal::zero(), "User need to repay loan");
        }

        // Check if the user belongs to this lending protocol

        pub fn check_user_exist(&self, user_badge: ResourceAddress) -> bool {
            if self.user_address == user_badge 
            {
                assert!(self.user_address == user_badge);
                return true 
            } else {
                return false
            };
        }

        // Adds the deposit balance
        // Checks if the user already a record of the resource or not
        pub fn add_deposit_balance(&mut self, user_auth: NonFungibleId, address: ResourceAddress, amount: Decimal) {
            let user_id = self.user_data.get(&user_auth).unwrap();

            // If the User already has the resource address, adds to the balance. If not, registers new resource address.
            let mut non_fungible_data: User = get_user.non_fungible().data();
            if non_fungible_data.deposit_balance.contains_key(&address) {
                *non_fungible_data.deposit_balance.get_mut(&address).unwrap() += amount;
            }     
            else {
                self.insert_deposit_resource(user_auth, address, amount);  
            };

            self.user_badge_vault.authorize(|| user_auth.non_fungible().update_data(non_fungible_data));
        }
        
        fn insert_deposit_resource(&mut self, user_auth: Proof, address: ResourceAddress, amount: Decimal) {

            let mut non_fungible_data: User = user_auth.non_fungible().data();
            non_fungible_data.deposit_balance.insert(address, amount);
        }

        // Checks the user's total tokens and deposit balance of those tokens


        // Grabs the deposit balance of a specific token
        pub fn check_token_deposit_balance(&self, token_address: ResourceAddress, user_auth: Proof) -> Decimal {
            let user_badge_data: User = user_auth.non_fungible().data();
            return *user_badge_data.deposit_balance.get(&token_address).unwrap();
        }

        pub fn deposit_resource_exists(&self, user_auth: Proof, address: ResourceAddress) -> bool {
            let user_badge_data: User = user_auth.non_fungible().data();
            return user_badge_data.deposit_balance.contains_key(&address);
        }

        pub fn assert_deposit_resouce_exists(&self, user_auth: Proof, address: ResourceAddress, label: String) {
            assert!(self.deposit_resource_exists(user_auth, address), "[{}]: No resource exists for user.", label);
        }

        pub fn assert_deposit_resouce_doesnt_exists(&self, user_auth: Proof, address: ResourceAddress, label: String) {
            assert!(!self.deposit_resource_exists(user_auth, address), "[{}]: Resource exists for user.", label);
        }

        // Adds the borrow balance
        // Checks if the user already a record of the resource or not
        pub fn add_borrow_balance(&mut self, user_auth: Proof, address: ResourceAddress, amount: Decimal) {

            let mut non_fungible_data: User = user_auth.non_fungible().data();
            if non_fungible_data.borrow_balance.contains_key(&address) {
                *non_fungible_data.borrow_balance.get_mut(&address).unwrap() += amount;
            }     
            else {
                self.insert_borrow_resource(user_auth, address, amount);
            };

            self.user_badge_vault.authorize(|| user_auth.non_fungible().update_data(non_fungible_data));
        }

        fn insert_borrow_resource(&mut self, user_auth: Proof, address: ResourceAddress, amount: Decimal) {

            let mut non_fungible_data: User = user_auth.non_fungible().data();
            non_fungible_data.borrow_balance.insert(address, amount);
        }

        pub fn check_borrow_balance(&self, user_auth: Proof) { // This way or check_deposit_balance?
            let user_badge_data: User = user_auth.non_fungible().data();
            for (token, amount) in &user_badge_data.borrow_balance {
                println!("{}: \"{}\"", token, amount)
            }
        }

        pub fn check_token_borrow_balance(&self, token_address: ResourceAddress, user_auth: Proof) -> Decimal {
            let user_badge_data: User = user_auth.non_fungible().data();
            return *user_badge_data.deposit_balance.get(&token_address).unwrap();
        }

        pub fn borrow_resource_exists(&self, user_auth: Proof, address: ResourceAddress) -> bool {
            let user_badge_data: User = user_auth.non_fungible().data();
            return user_badge_data.borrow_balance.contains_key(&address);
        }

        pub fn on_repay(&mut self, user_auth: Proof, token_address: ResourceAddress, repay_amount: Decimal) -> Decimal {
            let mut user_badge_data: User = user_auth.non_fungible().data();
            let borrow_balance: Decimal = self.check_token_borrow_balance(token_address, user_auth);
            if borrow_balance < repay_amount {
                let to_return = repay_amount - *user_badge_data.borrow_balance.get_mut(&token_address).unwrap();
                return to_return
            } else {
                *user_badge_data.borrow_balance.get_mut(&token_address).unwrap() -= repay_amount;
                Decimal::zero()
            }
        }

        pub fn check_collateral_ratio(&self, user_auth: Proof, token_address: ResourceAddress) -> Decimal {

            let collateral_ratio: Decimal = self.check_token_borrow_balance(token_address, user_auth) / self.check_token_deposit_balance(token_address, user_auth);
            return collateral_ratio
        }

        pub fn check_current_collateral_ratio(&self, user_auth: Proof, token_address: ResourceAddress, amount: Decimal) -> Decimal {
                  
            let collateral_ratio: Decimal = (self.check_token_borrow_balance(token_address, user_auth) + amount) / self.check_token_deposit_balance(token_address, user_auth);
            return collateral_ratio
        }
    }
}
