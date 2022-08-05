use scrypto::prelude::*;
use radex::radex::*;
use crate::price_oracle::*;

blueprint! {
    struct FundLocker {
        fund_vaults: HashMap<ResourceAddress, Vault>,
        token_weights: HashMap<ResourceAddress, Decimal>,
        fund_admin_vault: Vault,
        fund_token_address: ResourceAddress,
        starting_share_price: Decimal,
        price_oracle_address: ComponentAddress,
        radex_address: ComponentAddress,
    }

    impl FundLocker {
        
        pub fn new(
            fund_name: String,
            fund_ticker: String,
            starting_share_price: Decimal,
            tokens: HashMap<ResourceAddress, Decimal>,
            price_oracle_address: ComponentAddress,
            radex_address: ComponentAddress,
        ) -> ComponentAddress 
        {

            let fund_admin = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", format!("{} Admin Badge", fund_name))
                .metadata("symbol", "FAB")
                .metadata("description", "Component Admin authority")
                .initial_supply(1);

            let fund_token_address = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("name", format!("{} Tokens", fund_name))
                .metadata("symbol", format!("{}", fund_ticker))
                .metadata("description", "Tokens that represent ownerhip of the fund.")
                .mintable(rule!(require(fund_admin.resource_address())), LOCKED)
                .burnable(rule!(require(fund_admin.resource_address())), LOCKED)
                .no_initial_supply();
            
            let vault_amount = tokens.iter();
            let mut fund_vaults: HashMap<ResourceAddress, Vault> = HashMap::new();
            let mut token_weights: HashMap<ResourceAddress, Decimal> = HashMap::new();

            for (token, weight) in vault_amount {
                fund_vaults.insert(*token, Vault::new(*token));
                token_weights.insert(*token, *weight);
            };

            return Self {
                fund_vaults: fund_vaults,
                token_weights: token_weights,
                fund_admin_vault: Vault::with_bucket(fund_admin),
                fund_token_address: fund_token_address,
                starting_share_price: starting_share_price,
                price_oracle_address: price_oracle_address,
                radex_address: radex_address,
            }
            .instantiate()
            .globalize()
        }

        pub fn buy(
            &mut self,
            xrd: Bucket
        )  -> Bucket
        {
            assert_eq!(xrd.resource_address(), RADIX_TOKEN,
                "[Fund Locker]: You can only purchase Fund Tokens with XRD."
            );

            let token_address: ResourceAddress = xrd.resource_address();

            let output_token: ResourceAddress = self.fund_token_address; 
            let radex: RaDEX = self.radex_address.into();
            let return_bucket: Bucket = radex.swap(xrd, output_token);

            return_bucket
        }

        pub fn sell(
            &mut self,
            fund_token: Bucket
        ) -> Bucket
        {
            assert_eq!(fund_token.resource_address(), self.fund_token_address,
                "[Fund Locker]: You can only sell Fund Tokens."
            );

            let token_address = fund_token.resource_address();

            let output_token = RADIX_TOKEN;
            let radex: RaDEX = self.radex_address.into();
            let return_bucket: Bucket = radex.swap(fund_token, output_token);

            return_bucket   
        }

        pub fn issue_tokens(
            &mut self,
            mut tokens: Vec<Bucket>,
        ) -> Bucket
        {
            // Retrieves how many bucket of tokens are being passed.
            let number_of_tokens = tokens.len();

            let mut cumulative_token_amount: Decimal = Decimal::zero();
            let mut cumulative_value: Decimal = Decimal::zero();

            let tokens_iter = tokens.iter();

            let price_oracle: PriceOracle = self.price_oracle_address.into();

            for bucket in tokens_iter {
                let token_amount: Decimal = bucket.amount();
                let price: Decimal = price_oracle.get_price(bucket.resource_address());
                let token_value: Decimal = token_amount * price; 
                cumulative_token_amount += token_amount;
                cumulative_value += token_value;
            }

            info!("Amount of tokens: {:?}", cumulative_token_amount);
            info!("Value of tokens: {:?}", cumulative_value);
            

            let mut amount_to_mint: Decimal = Decimal::zero();

            let mut counter = 0;
            while counter < number_of_tokens {
                // Retrieves each bucket of tokens.
                let token_buckets: Option<Bucket> = tokens.pop();

                match token_buckets {
                    Some(token) => { // If a bucket exists...

                        assert_ne!(
                            borrow_resource_manager!(token.resource_address()).resource_type(), ResourceType::NonFungible,
                            "[Fund Locker]: Assets must be fungible."
                        );

                        let token_address: ResourceAddress = token.resource_address();

                        // Retrieves amount of each token.
                        let token_amount: Decimal = token.amount();

                        // * VALUES THE TOKEN RECEIVED * //
                        let price_oracle: PriceOracle = self.price_oracle_address.into();
                        let price: Decimal = price_oracle.get_price(token_address);
                        let token_value: Decimal = token_amount * price;

                        // * MINTS FUND TOKENS * //
                        let token_weight: Decimal = *self.token_weights.get(&token_address).unwrap();
                        let fund_tokens_to_mint: Decimal = token_value * token_weight;

                        amount_to_mint += fund_tokens_to_mint;

                        assert_eq!(self.fund_vaults.contains_key(&token_address), true,
                            "[Fund Locker]: This token does not belong to this fund."
                        );

                        let fund_vault: &mut Vault = self.fund_vaults.get_mut(&token_address).unwrap();

                        fund_vault.put(token);

                    }
                    None => {
                        
                        info!("[Fund Locker]: All tokens deposited!");

                    }
                }

                counter += 1;

                info!("[Fund Locker]: Fund tokens issues: {:?}", amount_to_mint);

                }

            if counter == number_of_tokens {
                let fund_token = self.fund_admin_vault.authorize(|| 
                    borrow_resource_manager!(self.fund_token_address).mint(amount_to_mint)
                );

                let price_oracle: PriceOracle = self.price_oracle_address.into();
                price_oracle.set_price(self.fund_token_address, Decimal::one());

                info!("[Fund Locker]: Amount of Fund Tokens issued: {:?}", amount_to_mint);

                fund_token
            } else {
                let empty_bucket = self.fund_admin_vault.take(0);

                empty_bucket
            }

        }

        pub fn redeem(
            &mut self,
            fund_token: Bucket,
        ) -> Vec<Bucket>
        {
            assert_eq!(fund_token.resource_address(), self.fund_token_address,
                "[Fund Locker]: You may only redeem fund tokens that belongs to this index."
            );

            let token_amount: Decimal = fund_token.amount();

            let price_oracle: PriceOracle = self.price_oracle_address.into();
            let token_price: Decimal = price_oracle.get_price(fund_token.resource_address());

            let token_value: Decimal = token_price * token_amount;

            let number_of_tokens: usize = self.token_weights.len();

            let mut token_addresses = self.fund_vaults.keys().cloned().collect::<Vec<ResourceAddress>>();
            let mut counter = 0;

            let mut return_collateral: Vec<Bucket> = Vec::new();

            while counter <= number_of_tokens {
                let collateral_token: Option<ResourceAddress> = token_addresses.pop();
                match collateral_token {
                    Some(token) => {
                        let collateral_weight: Decimal = *self.token_weights.get(&token).unwrap();
                        let collateral_price = price_oracle.get_price(token);
                        let collateral_value = collateral_price * collateral_weight;
                        let collateral_amount = token_value * collateral_value;

                        let collateral_bucket: Bucket = self.fund_vaults.get_mut(&token).unwrap().take(collateral_amount);
                        return_collateral.push(collateral_bucket);

                        counter += 1;
                        info!("[Redeem]: {:?} of {:?}", token, collateral_amount);
                        info!("[Redeem]: Counter: {:?}", counter);
                    }
                    None => {}
                }
            }
            let bucket_amount = return_collateral.len();
            info!("[Redeem]: Bucket: {:?}", bucket_amount);
            return_collateral

        }

        pub fn get_total_token_amount(
            &self,
            tokens: Vec<Bucket>
        ) -> Decimal
        {
            let mut amount = Decimal::zero();

            let buckets = tokens.iter();

            for token in buckets {
                let token_amount = token.amount();
                amount += token_amount;
            }

            amount
        }

        fn get_total_fund_tokens_issued(
            &self,
            tokens: &Vec<Bucket>
        ) -> Decimal
        {
            let mut amount = Decimal::zero();

            let buckets = tokens.iter();

            for token in buckets {
                let token_amount: Decimal = token.amount();
                let token_address: ResourceAddress = token.resource_address();
                let token_weight: Decimal = *self.token_weights.get(&token_address).unwrap();
                let price_oracle: PriceOracle = self.price_oracle_address.into();
                let token_price: Decimal = price_oracle.get_price(token_address);
                let token_value: Decimal = token_amount * token_price;
                let amount_minted: Decimal = token_value * token_weight;

                amount += amount_minted;
            }

            amount
        }

        fn withdraw(
            &mut self,
            token_address: ResourceAddress,
            amount: Decimal
        ) -> Bucket 
        {
            // Performing the checks to ensure tha the withdraw can actually go through
            // self.assert_belongs_to_pool(resource_address, String::from("Withdraw"));
            
            // Getting the vault of that resource and checking if there is enough liquidity to perform the withdraw.
            let fund_vault: &mut Vault = self.fund_vaults.get_mut(&token_address).unwrap();
            assert!(
                fund_vault.amount() >= amount,
                "[Withdraw]: Not enough liquidity available for the withdraw."
            );

            return fund_vault.take(amount);
        }

        pub fn swap(
            &mut self,
            collateral_address: ResourceAddress,
            amount: Decimal,
            output_token: ResourceAddress,
        ) -> Bucket
        {
            let input_bucket: Bucket = self.withdraw(collateral_address, amount);
            let radex: RaDEX = self.radex_address.into();
            let return_bucket = radex.swap(input_bucket, output_token);

            return_bucket
        }
    }
}