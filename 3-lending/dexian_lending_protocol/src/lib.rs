mod assetstate;
mod definterestmodel;
mod stableinterestmodel;
mod cdp;
mod oracle;

use scrypto::prelude::*;

use assetstate::*;
use cdp::*;


blueprint! {
    struct LendingPool {
        // asset price oracle
        oracle_addr: ComponentAddress,
        //Status of each asset in the lending pool
        states: HashMap<ResourceAddress, AssetState>,
        // address map for supply token(K) and deposit asset(V)
        origin_asset_map: HashMap<ResourceAddress, ResourceAddress>,
        // vault for each collateral asset(supply token)
        collateral_vaults: HashMap<ResourceAddress, Vault>,
        // Cash of each asset in the lending pool
        vaults: HashMap<ResourceAddress, Vault>,
        // CDP token define
        cdp_res_addr: ResourceAddress,
        // CDP id counter
        cdp_id_counter: u64,
        // lending pool admin badge.
        admin_badge: ResourceAddress,
        // minter
        minter: Vault,

    }

    impl LendingPool {
        
        pub fn instantiate_asset_pool(oracle_addr: ComponentAddress) -> (ComponentAddress, Bucket) {
            let admin_badge = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .initial_supply(dec!("1"));

            let minter = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .initial_supply(dec!("1"));
            
            let cdp_res_addr = ResourceBuilder::new_non_fungible()
                .metadata("symbol", "CDP")
                .metadata("name", "DeXian CDP Token")
                .mintable(rule!(require(minter.resource_address())), LOCKED)
                .burnable(rule!(require(minter.resource_address())), LOCKED)
                .no_initial_supply();
            
            let rules = AccessRules::new()
                .method("new_pool", rule!(require(admin_badge.resource_address())))
                // .method("withdraw_fees", rule!(require(admin_badge.resource_address())))
                .default(rule!(allow_all));

            // Instantiate a LendingPool component
            let component = LendingPool {
                states: HashMap::new(),
                origin_asset_map: HashMap::new(),
                collateral_vaults: HashMap::new(),
                vaults: HashMap::new(),
                cdp_id_counter: 0u64,
                minter: Vault::with_bucket(minter),
                admin_badge: admin_badge.resource_address(),
                cdp_res_addr,
                oracle_addr
            }
            .instantiate()
            .add_access_check(rules)
            .globalize();

            (component, admin_badge)
        }

        
        pub fn new_pool(&mut self, asset_address: ResourceAddress, 
            ltv: Decimal,
            liquidation_threshold: Decimal,
            liquidation_bonus: Decimal,
            insurance_ratio: Decimal, 
            interest_model: ComponentAddress) -> ResourceAddress  {
            let res_mgr = borrow_resource_manager!(asset_address);

            let origin_symbol = res_mgr.metadata()["symbol"].clone();
            let supply_token = ResourceBuilder::new_fungible()
                .metadata("symbol", format!("dx{}", origin_symbol))
                .metadata("name", format!("DeXian Lending LP token({}) ", origin_symbol))
                .mintable(rule!(require(self.minter.resource_address())), LOCKED)
                .burnable(rule!(require(self.minter.resource_address())), LOCKED)
                .no_initial_supply();
            
            let asset_state = AssetState {
                supply_index: Decimal::ONE,
                borrow_index: Decimal::ONE,
                borrow_interest_rate: Decimal::ZERO,
                supply_interest_rate: Decimal::ZERO,
                insurance_balance: Decimal::ZERO,
                token: supply_token,
                normalized_total_borrow: Decimal::ZERO,
                last_update_epoch: Runtime::current_epoch(),
                ltv,
                liquidation_threshold,
                liquidation_bonus,
                insurance_ratio,
                interest_model
            };

            self.states.insert(asset_address, asset_state);
            self.origin_asset_map.insert(supply_token, asset_address);
            self.vaults.insert(asset_address, Vault::new(asset_address));
            supply_token
        }

        pub fn supply(&mut self, deposit_asset: Bucket) -> Bucket {
            let asset_address = deposit_asset.resource_address();
            assert!(self.states.contains_key(&asset_address) && self.vaults.contains_key(&asset_address), "There is no pool of funds corresponding to the assets!");
            let asset_state = self.states.get_mut(&asset_address).unwrap();
            
            asset_state.update_index();

            let amount = deposit_asset.amount();
            let vault = self.vaults.get_mut(&asset_address).unwrap();
            vault.put(deposit_asset);

            let normalized_amount = LendingPool::floor(amount / asset_state.supply_index);
            
            let supply_token = self.minter.authorize(|| {
                let supply_res_mgr: &ResourceManager = borrow_resource_manager!(asset_state.token);
                supply_res_mgr.mint(normalized_amount)
            });

            asset_state.update_interest_rate();
            
            supply_token
        }

        pub fn withdraw(&mut self, supply_token: Bucket) -> Bucket {
            let token_address = supply_token.resource_address();
            assert!(self.origin_asset_map.contains_key(&token_address), "unsupported the token!");
            let amount = supply_token.amount();
            let asset_address = self.origin_asset_map.get(&token_address).unwrap();
            let asset_state = self.states.get_mut(&asset_address).unwrap();

            asset_state.update_index();

            let normalized_amount = LendingPool::floor(amount * asset_state.supply_index);
            self.minter.authorize(|| {
                let supply_res_mgr: &ResourceManager = borrow_resource_manager!(asset_state.token);
                supply_res_mgr.burn(supply_token);
            });
            let vault = self.vaults.get_mut(&asset_address).unwrap();
            let asset_bucket = vault.take(normalized_amount);
            asset_state.update_interest_rate();
            //TODO: log
            asset_bucket
        }

        pub fn borrow(&mut self, supply_token: Bucket, borrow_token: ResourceAddress, mut amount: Decimal) -> (Bucket, Bucket){
            assert!(self.states.contains_key(&borrow_token), "unsupported the borrow token!");
            let token_address = supply_token.resource_address();
            assert!(self.origin_asset_map.contains_key(&token_address), "unsupported the collateral token!");
            
            let collateral_addr = self.origin_asset_map.get(&token_address).unwrap();
            debug!("borrow supply_token {}, collateral_addr {}, ", token_address, collateral_addr);
            let collateral_state = self.states.get_mut(collateral_addr).unwrap();
            assert!(collateral_state.ltv > Decimal::ZERO, "Then token is not colleteral asset!");
            
            collateral_state.update_index();
            
            let supply_index = collateral_state.supply_index;
            let ltv = collateral_state.ltv;
            let supply_amount = supply_token.amount();

            let deposit_amount = LendingPool::floor(supply_amount * supply_index);
            let max_loan_amount = self.get_max_loan_amount(collateral_addr.clone(), deposit_amount, ltv, borrow_token);
            debug!("max loan amount {}, supply_amount:{} deposit_amount:{}, amount:{}", max_loan_amount, supply_amount, deposit_amount, amount);
            if amount > max_loan_amount {
                amount = max_loan_amount;
            }

            if self.collateral_vaults.contains_key(&token_address){
                let collateral_vault = self.collateral_vaults.get_mut(&token_address).unwrap();
                collateral_vault.put(supply_token);
            }
            else{
                let vault = Vault::with_bucket(supply_token);
                self.collateral_vaults.insert(token_address, vault);
            }

            
            let borrow_asset_state = self.states.get_mut(&borrow_token).unwrap();
            borrow_asset_state.update_index();
            
            let borrow_normalized_amount = LendingPool::ceil(amount / borrow_asset_state.borrow_index);
            borrow_asset_state.normalized_total_borrow += borrow_normalized_amount;
            borrow_asset_state.update_interest_rate();

            let borrow_vault = self.vaults.get_mut(&borrow_token).unwrap();
            let borrow_bucket = borrow_vault.take(amount);

            let data = CollateralDebtPosition{
                collateral_token: collateral_addr.clone(),
                total_borrow: amount,
                total_repay: Decimal::ZERO,
                normalized_borrow: borrow_normalized_amount,
                collateral_amount: supply_amount,
                borrow_amount: amount,
                last_update_epoch: Runtime::current_epoch(),
                borrow_token
            };

            let cdp = self.minter.authorize(|| {
                self.cdp_id_counter += 1;
                let cdp_res_mgr: &ResourceManager = borrow_resource_manager!(self.cdp_res_addr);
                cdp_res_mgr.mint_non_fungible(&NonFungibleId::from_u64(self.cdp_id_counter), data)
            });
            (borrow_bucket, cdp)
        }

        pub fn repay(&mut self, mut repay_token: Bucket, cdp: Bucket) -> (Bucket, Option<Bucket>) {
            assert!(
                cdp.amount() == dec!("1"),
                "We can only handle one CDP each time!"
            );

            let cdp_id = cdp.non_fungible::<CollateralDebtPosition>().id();
            let mut cdp_data: CollateralDebtPosition = cdp.non_fungible().data();
            let borrow_token = cdp_data.borrow_token;
            assert!(borrow_token == repay_token.resource_address(), "Must return borrowed coin.");

            let borrow_state = self.states.get_mut(&borrow_token).unwrap();
            debug!("before update_index, borrow normalized:{} indexes:{},{}", cdp_data.normalized_borrow, borrow_state.supply_index, borrow_state.borrow_index);
            borrow_state.update_index();
            debug!("before update_index, borrow normalized:{} indexes:{},{}", cdp_data.normalized_borrow, borrow_state.supply_index, borrow_state.borrow_index);
            let borrow_index = borrow_state.borrow_index;
            assert!(borrow_index > Decimal::ZERO, "borrow index error! {}", borrow_index);
            let normalized_amount = LendingPool::floor(repay_token.amount() / borrow_index);
            let mut repay_amount = repay_token.amount();

            let borrow_vault = self.vaults.get_mut(&borrow_token).unwrap();
            borrow_vault.put(repay_token.take(repay_amount));

            let mut collateral_bucket: Option<Bucket> = None;
            if cdp_data.normalized_borrow <= normalized_amount {
                // repayAmount <= amount
                // because ⌈⌊a/b⌋*b⌉ <= a
                repay_amount = LendingPool::ceil(cdp_data.normalized_borrow * borrow_index);

                let collateral_token = cdp_data.collateral_token;
                let collateral_vault = self.collateral_vaults.get_mut(&collateral_token).unwrap();
                collateral_bucket = Some(collateral_vault.take(cdp_data.collateral_amount));
                
                // cdp_data.normalized_borrow = Decimal::ZERO;
                // cdp_data.collateral_amount = Decimal::ZERO;
                // cdp_data.total_repay += repay_amount;
                // cdp_data.last_update_epoch = Runtime::current_epoch();
                self.minter.authorize(|| {
                    // let cdp_res_mgr: &ResourceManager = borrow_resource_manager!(cdp.resource_address());
                    // cdp_res_mgr.update_non_fungible_data(&cdp_id, cdp_data);
                    cdp.burn();
                });

                return (repay_token, collateral_bucket);
            }

            cdp_data.total_repay += repay_amount;
            cdp_data.normalized_borrow -= normalized_amount;
            cdp_data.last_update_epoch = Runtime::current_epoch();
            self.minter.authorize(|| {
                let cdp_res_mgr: &ResourceManager = borrow_resource_manager!(cdp.resource_address());
                cdp_res_mgr.update_non_fungible_data(&cdp_id, cdp_data);
            });

            (cdp, collateral_bucket)
            
        }

        pub fn get_asset_price(&self, asset_addr: ResourceAddress) -> Decimal{
            let component: &Component = borrow_component!(self.oracle_addr);
            component.call::<Decimal>("get_price_quote_in_xrd", args![asset_addr])
        }

        fn get_max_loan_amount(&self, deposit_asset: ResourceAddress, deposit_amount: Decimal, ltv: Decimal, borrow_asset: ResourceAddress) -> Decimal{
            deposit_amount * self.get_asset_price(deposit_asset) * ltv / self.get_asset_price(borrow_asset)
        }

        fn ceil(dec: Decimal) -> Decimal{
            dec.round(18u8, RoundingMode::TowardsPositiveInfinity)
        }

        fn floor(dec: Decimal) -> Decimal{
            dec.round(18u8, RoundingMode::TowardsNegativeInfinity)
        }
    }
}
