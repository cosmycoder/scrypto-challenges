/// **This submission does not need to be judged due to being incomplete***
/// *** This is copypasta from liquidity dao example from github and I am in 
/// progress of altering for my specific needs***
/// 
use scrypto::prelude::*;

#[derive(TypeId, Encode, Decode, Describe, Debug, PartialEq)]
pub enum Vote { // Choices users can make to cast a vote.
    Yes, // The `Yes` vote signifies support
    No, // The `No` vote signifies disagreement of the proposal.
    YesWithChallenge, // The `Challenge` vote signifies a significantly stronger opinion you are willing to risk your reputation on
    NoWithChallenge, // The `Challenge` vote signifies a significantly stronger opinion you are willing to risk your reputation on
}

#[derive(TypeId, Encode, Decode, Describe, Debug, PartialEq)]
pub enum Resolution { // This enum descibes the states in which the governance proposal can be in.
    Passed, // `Passed` allows the proposal to change the pool parameters.
    Failed, // `Failed` does nothing and burns the Proposal NFT.
    InProcess, // `InProcess` signifies that the Proposal NFT is still being proposed or voted on.
}

#[derive(TypeId, Encode, Decode, Describe, Debug, PartialEq)]
pub enum Stage { // This enum describes the stage of the governance proposal.
    DepositPhase, // Stage where funds are collected to advance to voting
    VotingPhase, // Stage where XRDao users can vote on the outcome of proposal
    ChallengePhase, // Stage that means the proposal has been challenged with different requirements to Pass or Fail
}

/// This struct describes the data that the Proposal NFT holds.
#[derive(NonFungibleData, Debug)]
pub struct Proposal {
    #[scrypto(mutable)]
    pub new_investment_lp_component_address: ComponentAddress,
    #[scrypto(mutable)]
    pub new_percent_reward_to_lp: Decimal,
    #[scrypto(mutable)]
    pub new_sell_fee: Decimal,
    #[scrypto(mutable)]
    pub new_buy_fee: Decimal,
    #[scrypto(mutable)]
    pub amount_deposited: Decimal,
    #[scrypto(mutable)]
    pub stage: Stage,
    #[scrypto(mutable)]
    pub yes_count: Decimal,
    #[scrypto(mutable)]
    pub no_count: Decimal,
    #[scrypto(mutable)]
    pub yes_with_challenge_count: Decimal,
    #[scrypto(mutable)]
    pub no_with_challenge_count: Decimal,
    pub vote_ends_in_epoch: u64,
    pub proposal_end_epoch: u64,
    pub current_epoch: u64,
    #[scrypto(mutable)]
    pub resolution: Resolution,
}

blueprint!{
    struct XrdaoProposal {
        nft_proposal_admin: Vault, // This contains the admin badge to which Proposal NFTs and Vote Badge NFTs can be minted, burnt, or updated.
        nft_proposal_address: ResourceAddress, // Resource address of the Proposal NFT used primarily to view the NFT data or prove that it belongs to this protocol.
        vote_receipt: ResourceAddress, // Resource address of the NFT vote receipt given to users for each vote
        voting_yes_vault: Vault, // This is the vault where LP Tokens are allocated towards a `Yes` vault.
        voting_no_vault: Vault, // This is the vault where LP Tokens are allocated towards a `No` vault.
        voting_yes_challenge_vault: Vault, // This is the vault where LP Tokens are allocated towards a `Yes with challnege` vault.
        voting_no_challenge_vault: Vault, // This is the vault where LP Tokens are allocated towards a `No with challnege` vault.
        voting_end_epoch: u64, // This is the time constraint for the Voting Period.
        xrdao_quorom: Decimal, // This is the minimum amount of votes that need to resolve the proposal.
        rep_quorom: Decimal, // This is the minimum amount of votes that need to resolve the proposal.
        proposal_funding_amount: Decimal, // This is the minimum XRD required to advance the proposal from the Deposit Phase to the Voting Period stage.
        proposal_end_epoch: u64, // This is the time constraint for the Deposit Phase.
        proposal_in_voting_period: Vec<NonFungibleId>, // This is where proposal(s) who have advanced to the Voting Period will be contained. Only one can advance to the Voting Period at a time. 
        proposal_record: Option<ComponentAddress>, // This is the component address of the liquidity pool to change the pool parameter if the proposal succeeds.
    }

    impl XrdaoProposal {
    
        pub fn new(tracking_token_address: ResourceAddress, nft_proposal_admin: Bucket) -> ComponentAddress {
            // The Proposal NFT definition
            let nft_proposal = ResourceBuilder::new_non_fungible()
                .metadata("name", "XRDao Proposal")
                .metadata("symbol", "XRDP")
                .metadata("description", "An NFT that documents governance proposal for XRDao")
                .mintable(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .burnable(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .no_initial_supply();

            let vote_receipt: ResourceBuilder::new_non_fungible()
                .metadata("name", "XRDao Vote Receipt")
                .metadata("symbol", "XRDVR")
                .metadata("description", "An NFT that stores vote record")
                .mintable(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .burnable(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(nft_proposal_admin.resource_address())), LOCKED)
                .no_initial_supply();
            
            // Creating the liquidity pool DAO component and instantiating it
            let xrdao_proposal: ComponentAddress = Self { 
                nft_proposal_admin: Vault::with_bucket(nft_proposal_admin),
                nft_proposal_address: nft_proposal,
                proposal_vault: Vault::new(nft_proposal),
                voting_yes_vault: Vault::new(tracking_token_address),
                voting_no_vault: Vault::new(tracking_token_address),
                voting_yes_challenge_vault: Vault::new(tracking_token_address),
                voting_no_challenge_vault: Vault::new(tracking_token_address),
                voting_end_epoch: 10,
                xrdao_quorom: dec!("0.3"),
                rep_quorom: dec!("0.67"),
                minimum_xrd: dec!("1000"),
                xrd_vault: Vault::new(RADIX_TOKEN),
                proposal_end_epoch: 5,
                proposal_in_voting_period: Vec::new(),
                liquidity_pool_address: None,
            }
            .instantiate() // ************************************************************

            xrdao_proposal
        }
        
        /// Creates a governance proposal
        /// * **Check 1:** Checks that the weight of both tokens cannot exceed 100%.
        /// * **Check 2:** Checks that the fee must be between 0% and 100%.
        /// # Arguments:
        /// * `lp_proof` (Proof) - Proof of the LP Token to determine if the proposer is an LP.
        /// * `token_1_weight` (Decimal) - The weight desired to change for Token 1.
        /// * `token_2_weight` (Decimal) - The weight desired to change for Token 2.
        /// * `fee_to_pool` (Decimal) - The swap fee desired to change for the pool.
        /// # Returns:
        /// `NonFungible` - The NonFungibleId of the minted Proposal NFT used for identification. 
        pub fn propose(&mut self, user_id: Proof, token_1_weight: Decimal, token_2_weight: Decimal, fee_to_pool: Decimal) -> NonFungibleId {
            
            // let liquidity_pool: LiquidityPoolComponent = self.liquidity_pool_address.unwrap().into();
            
            // // Retrieves the resource address of the tracking_token
            // let tracking_token_address = liquidity_pool.tracking_token_address();

            // lp_proof.validate_proof(
            //     ProofValidationMode::ValidateResourceAddress(tracking_token_address))
            //     .expect("[Governance Proposal]: LP Token does not belong to this liquidity pool");
                
            //     assert!((token_1_weight + token_2_weight) <= dec!("1.0"), 
            //         "[Governance Proposal]: The weight of both tokens cannot exceed {:?}.", dec!("1.0"));
            //     assert!((fee_to_pool >= Decimal::zero()) & (fee_to_pool <= dec!("100")), 
            //         "[Governance Proposal]: Fee must be between 0 and 100");

            // Mints the Proposal NFT based on the arguments passed.
            let nft_proposal = self.nft_proposal_admin.authorize(|| {
                borrow_resource_manager!(self.nft_proposal_address)
                .mint_non_fungible(
                    &NonFungibleId::random(),
                    Proposal {
                        token_1_weight: token_1_weight,
                        token_2_weight: token_2_weight,
                        fee_to_pool: fee_to_pool,
                        amount_deposited: Decimal::zero(),
                        yes_counter: Decimal::zero(),
                        no_counter: Decimal::zero(),
                        no_with_veto_counter: Decimal::zero(),
                        no_with_veto_counter: Decimal::zero(),
                        stage: Stage::DepositPhase,
                        vote_ends_in_epoch: self.voting_end_epoch,
                        proposal_end_epoch: self.proposal_end_epoch,
                        current_epoch: Runtime::current_epoch(),
                        resolution: Resolution::InProcess,
                        
                        voting_end_epoch: 10,
                        xrdao_quorom: dec!("0.5"),
                        rep_quorom: dec!("0.67"),
                        minimum_xrd: dec!("1000"),
                        xrd_vault: Vault::new(RADIX_TOKEN),
                        proposal_end_epoch: 5,
                        proposal_in_voting_period: Vec::new(),
                        liquidity_pool_address: None,
                    },
                )
            });

            // Calls the liquidity pool component to allow liquidity pool visibility that the Proposal NFT has been created.
            self.push_nft_proposal_address();

            // Retrieves the LiquidityPool component.
            let liquidity_pool: LiquidityPoolComponent = self.liquidity_pool_address.unwrap().into();

            // Retrieves LiquidityPool component data.
            let liquidity_pool_token_1_weight: Decimal = liquidity_pool.token_1_weight();

            let liquidity_pool_token_2_weight: Decimal = liquidity_pool.token_2_weight();

            let liquidity_pool_fee_to_pool: Decimal = liquidity_pool.fee_to_pool();

            // Retrieves the Proposal NFT ID.
            let proposal_id = nft_proposal.non_fungible::<Proposal>().id();

            info!("[Governance Proposal]: Proposal has has been created!");
            info!("[Governance Proposal]: The proposal ID is {:?}", proposal_id);
            info!("[Governance Proposal]: Token 1 weight adjustment to {:?} from {:?}",
                token_1_weight, liquidity_pool_token_1_weight);
            info!("[Governance Proposal]: Token 2 weight adjustment to {:?} from {:?}",
                token_2_weight, liquidity_pool_token_2_weight);
            info!("[Governance Proposal]: Pool fee adjustment to {:?} from {:?}",
                fee_to_pool, liquidity_pool_fee_to_pool);   
            info!("[Governance Proposal]: The conditions to advance this proposal to the Voting Period are as follows:"); 
            info!("[Governance Proposal]: Condition 1 - A minimum of {:?} in XRD must be deposited.", 
                self.minimum_xrd);
            info!("[Governance Proposal]: The Voting Period will end in {:?} epoch.", self.proposal_end_epoch);

            // Puts the Proposal NFT ID in its respective vault.
            self.proposal_vault.put(nft_proposal);

            return proposal_id
        }

        /// This method is used to deposit XRD towards the Proposal NFT in the Deposit Phase.
        /// This method performs a few checks:
        /// * **Check 1:** Checks that the proposal exists.
        /// * **Check 2:** Checks that the bucket passed contains XRD.
        /// * **Check 3:** Checks that the user has not deposited more than what is required to advance the proposal.
        /// * **Check 4:** Checks that no more than one Proposal NFT can be advanced to the Voting Period.
        /// # Arguments:
        /// * `fee` (Bucket) - The bucket that contains the XRD to be deposited towards the Proposal NFT.
        /// * `proposal_id` (NonFungibleId) - The NonFungibleId of the proposal selected.
        /// This method does not return any assets.
        pub fn deposit_proposal(&mut self,fee: Bucket, proposal_id: NonFungibleId) {

            assert_eq!(self.proposal_vault.non_fungible_ids().contains(&proposal_id), true, 
                "The proposal you are voting for does not exist.");

            assert_eq!(fee.resource_address(), self.xrd_vault.resource_address(), 
                "Incorrect asset deposited.");

            let amount = fee.amount();

            // Retrieves resource manager.
            // We use the resource manager to update the Proposal data NFT because it's a common
            // occurence to forget to put the NFT back into the vault after it's been updated.
            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);
            let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

            proposal_data.amount_deposited += amount;

            // Since proposal_data.amount_deposited value has been used, we shall create another one.
            // We also want to have this after the amount has been changed so we can retrieve the updated data.
            let amount_deposited = proposal_data.amount_deposited;

            assert!(amount_deposited <= self.minimum_xrd, 
                "You have deposited {:?} more XRD than what is required for this proposal.",
                (amount_deposited - self.minimum_xrd)
            );

            // Authorize update of the Proposal NFT.
            self.nft_proposal_admin.authorize(|| 
                resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
            );

            self.xrd_vault.put(fee);

            // Retrieves the Proposal NFT data again since the previous value has been consumed.
            let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

            // The scenario when the Proposal NFT has met its deposit quota. It can now be advanced.
            if proposal_data.amount_deposited >= self.minimum_xrd {

                // Retrieves the Proposal NFT data again since the previous value has been consumed.
                let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                proposal_data.stage = Stage::VotingPeriod;

                // Authorize update of the Proposal NFT.
                self.nft_proposal_admin.authorize(|| 
                    resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                );

                // This helper function is used to ensure that no more than one Proposal NFT has advanced to the Voting Period.
                self.check_voting_stage();

                // Pushes the Proposal NFT that has advanced to the Voting Period.
                self.proposal_in_voting_period.push(proposal_id);

                assert_eq!(self.proposal_in_voting_period.len(), 1, "Cannot have more than one proposal in the voting period.");

                info!("[Depositing Proposal]: This proposal has now advanced to the Voting Period phase!"); 
                info!("[Depositing Proposal]: The conditions to pass this proposal are as follows:"); 
                info!("[Depositing Proposal]: Condition 1 - Simple majority will need to vote 'Yes' in order to pass this proposal");
                info!("[Depositing Proposal]: Condition 2 - A minimum of {:?} of the protocol's voting power (quorum) is required.",
                    self.vote_quorom
                );
                info!("[Depositing Proposal]: Condition 3 - Less than {:?} of participating voting power votes 'no-with-veto'.",
                    self.no_with_veto_threshold
                );

            // The scneario when the Proposal NFT Deposit Phase has lapsed. It will be burnt.
            } else if Runtime::current_epoch() > self.proposal_end_epoch {

                // Here we take the NFT out of the vault because there is no other option other than
                // to burn the NFT.
                let proposal = self.proposal_vault.take_non_fungible(&proposal_id);

                info!("[Depositing Proposal]: The Deposit Phase has elapsed and this proposal is rejected.");
                info!("[Depositing Proposal]: Create a new proposal to start another process."); 

                self.nft_proposal_admin.authorize(|| proposal.burn());
            
            // The scenario when the Proposal NFT has not met its deposit quote yet and there is still time remaining.
            } else {

                let amount_deposited = proposal_data.amount_deposited;

                info!("[Depositing Proposal]: You have deposited {:?} XRD to this proposal.",
                    amount
                );

                info!("[Depositing Proposal]: This proposal has {:?} XRD deposited.", 
                    amount_deposited
                ); 

                info!("[Depositing Proposal]: This proposal require(s) additional {:?} XRD to advance this proposal to the Voting Period.", 
                    (self.minimum_xrd - amount_deposited)
                ); 
            };
        }

        /// This is a helper function to ensure that there is no more than one Proposal NFT that can be advanced.
        fn check_voting_stage(&self) {

            // Reviews all the NFTs within the `proposal_vault`.
            let proposals = self.proposal_vault.non_fungible_ids();

            // Creates an iterator to go through each NFTs.
            let all_proposals = proposals.iter();
            
            // Runs a for loop to review the stage of each NFT.
            for proposal in all_proposals {
                let resource_manager = borrow_resource_manager!(self.nft_proposal_address);
                let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal);
                let proposal_stage = proposal_data.stage;
                if proposal_stage == Stage::VotingPeriod {
                    let proposals_in_voting_period = vec![proposal];
                    assert_eq!(proposals_in_voting_period.len(), 1, "Cannot have more than one proposal in the voting period.");
                }
            }
        }

        /// This method allows users to check the status of a proposal.
        /// This method performs a single check:
        /// * **Check 1:** Checks whether the proposal exists.
        /// # Arguments:
        /// * `proposal_id` (NonFungibleId) - The NonfungibleId of the Proposal NFT the user wish to retrieve information.
        /// This method returns the information of the Proposal NFT.
        pub fn check_proposal(&self, proposal_id: NonFungibleId) {
            
            assert_eq!(self.proposal_vault.non_fungible_ids().contains(&proposal_id), true, 
                "The proposal you are attempint to retrieve information does not exist.");

            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);
            let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);
            info!("[Proposal Info]: Proposal ID: {:?}", proposal_id);
            info!("[Proposal Info]: Token 1 weight: {:?}", proposal_data.token_1_weight);
            info!("[Proposal Info]: Token 2 weight: {:?}", proposal_data.token_2_weight);
            info!("[Proposal Info]: Fee to Pool: {:?}", proposal_data.fee_to_pool);
            info!("[Proposal Info]: Amount required to be deposited to advance this proposal: {:?}", self.minimum_xrd);
            info!("[Proposal Info]: Deposit resource: {:?}", self.xrd_vault.resource_address());
            info!("[Proposal Info]: Amount deposited to this proposal: {:?}", proposal_data.amount_deposited);
            info!("[Proposal Info]: Yes: {:?}", proposal_data.yes_counter);
            info!("[Proposal Info]: No: {:?}", proposal_data.no_counter);
            info!("[Proposal Info]: No with veto: {:?}", proposal_data.no_with_veto_counter);
            info!("[Proposal Info]: Abstain: {:?}", 
                (
                    dec!("1.0") - (proposal_data.yes_counter + proposal_data.no_counter + proposal_data.no_with_veto_counter)
                )
            );
            info!("[Proposal Info]: Current stage of the proposal: {:?}", proposal_data.vote_ends_in_epoch);
            info!("[Proposal Info]: Deposit Phase period ends in: {:?} epoch", proposal_data.stage);
            info!("[Proposal Info]: Voting period ends in: {:?} epoch", proposal_data.proposal_end_epoch);
            info!("[Proposal Info]: Current epoch: {:?}", proposal_data.current_epoch);
            info!("[Proposal Info]: Resolution: {:?}", proposal_data.resolution);
        }
        
        /// This method simply is used to view the Proposal NFT that is in the Voting Period.
        /// This method does not perform any checks.
        /// This method does not accept any arguments.
        /// # Returns:
        /// * `Vec<NonFungibleId>` - The NonFungibleId of the Proposal NFT is a `Vec`.
        pub fn see_proposal(&self) -> Vec<NonFungibleId> {
            return self.proposal_in_voting_period.clone()
        }

        /// This method allows users to vote on a governance proposal.
        /// It takes either a `Yes`, `No`, or `No with veto` vote and matches each scenario appropriately.
        /// A Vote Badge NFT is then minted as a receipt of the vote.
        /// This method performs a few checks:
        /// * **Check 1:** Checks that the Proposal NFT exists.
        /// * **Check 2:** Checks that the correct LP Tokens have been passed.
        /// * **Check 3:** Checks whether the Proposal NFT has advanced to the Voting Period stage.
        /// * **Check 4:** Checks that the Proposal NFT has advanced to the Voting Period stage... but in a different way.
        /// # Arguments:
        /// * `lp_tokens` (Bucket) - The bucket that contains the LP Tokens used to vote.
        /// * `vote_submission` (Enum) - The finite selection of votes the user can select.
        /// * `proposal_id` (NonFungibleId) - The NonFungibleId of the Proposal NFT.
        /// # Returns:
        /// * `Bucket` - The Vote Badge NFT as a receipt of the user's vote.
        /// # Note:
        /// * I've used fungible LP Tokens as a way to cast votes. I understand that this can create a situation in which LPs can disperse their LP Tokens 
        /// across multiple votes. Although, I did not find a good reason to prevent LPs from doing that since doing so seems moot. Votes counted this way is 
        /// quite elegant as the LPs receive LP Token that represent their ownership of the liquidity pool. Therefore, their votes will be weighted as such, giving 
        /// more influence to those who have more skin in the game. 
        /// * Also note that there are two counters. One are the vaults where LP Tokens are allocated towards with each respective vote. The second are the data fields
        /// from the Proposal NFT to count votes. However, the vote vaults only count nominal amount and the Proposal NFT data field represents weighted amounts.   
        pub fn vote_proposal(&mut self, lp_tokens: Bucket, vote_submission: Vote, proposal_id: NonFungibleId) -> Bucket {
            // Retrieves the LiquidityPool component. 
            let liquidity_pool: LiquidityPoolComponent = self.liquidity_pool_address.unwrap().into();

            // Retrieves the resource address of the tracking_token
            let tracking_token_address = liquidity_pool.tracking_token_address();

            assert_eq!(self.proposal_vault.non_fungible_ids().contains(&proposal_id), true, 
                "The proposal you are voting for does not exist."
            );

            assert_eq!(lp_tokens.resource_address(), tracking_token_address,
                "Wrong LP tokens passed."
            );

            // Retrieves the resource manager.
            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);

            // Retrieves the Proposal NFT data.
            let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

            assert_eq!(proposal_data.stage, Stage::VotingPeriod, 
                "[Voting]: The proposal has not advanced to the Voting Period, yet.
                This proposal requires an additional deposit of {:?} in order to advance.",
                (self.minimum_xrd - proposal_data.amount_deposited)
            );

            assert_eq!(self.proposal_in_voting_period.contains(&proposal_id), true,
                "[Voting]: The proposal has not advanced to the Voting Period, yet."
            );
            
            // Get amount of voting tokens
            let amount = lp_tokens.amount();

            // Calculate vote weight.
            let tracking_tokens_manager: &ResourceManager = borrow_resource_manager!(tracking_token_address);
            let vote_weight: Decimal = amount / tracking_tokens_manager.total_supply();

            // Matches the `vote_submission` to its respective selections and mints a Vote Badge NFT to represent the selection.
            match vote_submission {
                // The scenario when the user votes `Yes` to the proposal.
                Vote::Yes => {
                    // Puts the LP Tokens the vault responsible for `Yes` votes.
                    self.voting_yes_vault.put(lp_tokens);

                    // Increases the yes counter for the Proposal NFT.
                    // There are two counters technically. 
                    proposal_data.yes_counter += vote_weight;

                    // Authorizes the Proposal NFT data update.
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    // Retrieves Proposal NFT data.
                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);

                    info!("[Voting]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                        yes_counter, no_counter, no_with_veto_counter, abstain_counter
                    );
                    info!("[Voting]: You have voted 'Yes' for the proposal.");
                    info!("[Voting]: The weight of your vote is {:?}.", vote_weight);

                    // Mints the Vote Badge NFT.
                    let vote_badge = self.nft_proposal_admin.authorize(|| {
                        borrow_resource_manager!(self.vote_badge_address)
                        .mint_non_fungible(
                            &NonFungibleId::random(),
                            VoteBadge {
                                proposal: proposal_id,
                                vote: Vote::Yes,
                                vote_weight: vote_weight,
                                lp_tokens_allocated: amount,
                            },
                        )
                    });

                    info!("[Voting]: You've received a badge for your vote!");
                    info!("[Voting]: Your badge proof is {:?}", vote_badge.resource_address());

                    return vote_badge
                }
                // The scenario when the user votes `No` to the proposal.
                Vote::No => {
                    
                    self.voting_no_vault.put(lp_tokens);

                    proposal_data.no_counter += vote_weight;

                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;

                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Voting]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                        yes_counter, no_counter, no_with_veto_counter, abstain_counter
                    );
                    info!("[Voting]: You have voted 'No' for the proposal.");
                    info!("[Voting]: The weight of your vote is {:?}.", vote_weight);

                    let vote_badge = self.nft_proposal_admin.authorize(|| {
                        borrow_resource_manager!(self.vote_badge_address)
                        .mint_non_fungible(
                            &NonFungibleId::random(),
                            VoteBadge {
                                proposal: proposal_id,
                                vote: Vote::No,
                                vote_weight: vote_weight,
                                lp_tokens_allocated: amount,
                            },
                        )
                    });

                    info!("[Voting]: You've received a badge for your vote!");
                    info!("[Voting]: Your badge proof is {:?}", vote_badge.resource_address());

                    return vote_badge
                }
                Vote::NoWithVeto => {
                    // The scenario when the user votes `No with veto` to the proposal.
                    self.voting_no_with_veto_vault.put(lp_tokens);

                    proposal_data.no_with_veto_counter += vote_weight;

                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);
                    
                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;
    
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Voting]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                        yes_counter, no_counter, no_with_veto_counter, abstain_counter
                    );

                    info!("[Voting]: You have voted 'No' for the proposal.");
                    info!("[Voting]: The weight of your vote is {:?}.", vote_weight);

                    let vote_badge = self.nft_proposal_admin.authorize(|| {
                        borrow_resource_manager!(self.vote_badge_address)
                        .mint_non_fungible(
                            &NonFungibleId::random(),
                            VoteBadge {
                                proposal: proposal_id,
                                vote: Vote::NoWithVeto,
                                vote_weight: vote_weight,
                                lp_tokens_allocated: amount,
                            },
                        )
                    });

                    info!("[Voting]: You've received a badge for your vote!");
                    info!("[Voting]: Your badge proof is {:?}", vote_badge.resource_address());

                    return vote_badge
                }
            };
        }

        /// This method is used to retrieve information from the Vote Badge NFT.
        /// This method performs a single check:
        /// * **Check 1:** Checks that the Vote Badge NFT belongs to this protocol.
        /// # Arguments:
        /// * `vote_badge` (Proof) - The proof of the Vote Badge NFT.
        /// Returns the Vote Badge NFT information.
        pub fn check_vote(&self, vote_badge: Proof) {

            let validated_vote_badge: ValidatedProof = vote_badge
            .validate_proof(
                ProofValidationMode::ValidateResourceAddress(self.vote_badge_address)
            )
            .expect("[Vote Info]: Invalid Proof provided.");
            
            let vote_badge_data = validated_vote_badge.non_fungible::<VoteBadge>().data();
            let proposal = vote_badge_data.proposal;
            let vote = vote_badge_data.vote;
            let vote_weight = vote_badge_data.vote_weight;
            let lp_tokens_allocated = vote_badge_data.lp_tokens_allocated;
            info!("[Vote Info]: Proposal: {:?}", proposal);
            info!("[Vote Info]: Vote: {:?}", vote);
            info!("[Vote Info]: Vote weight: {:?}", vote_weight);
            info!("[Vote Info]: LP tokens allocated: {:?}", lp_tokens_allocated);
        }

        /// This method allows user to recast their vote.
        /// It requires a lot of coordination to ensure the data is manipulated correctly.
        /// The method uses a helper function to organize and assist the faciliation of the movement of assets.
        /// This method performs a single check:
        /// * **Check 1:** Checks that the Vote Badge NFT belongs to this protocol.
        /// * **Check 2:** Checks that the vote recasted is not the same as the current vote casted.
        /// # Arguments:
        /// * `vote_badge` (Proof) - The proof of the Vote Badge NFT.
        /// * `vote_submission` (Enum) - The choice of vote the user wants to recast.
        /// This method does not return any assets, but will change their Vote Badge NFT along with the 
        /// Proposal NFT. 
        pub fn recast_vote(&mut self, vote_badge: Proof, vote_submission: Vote) {

            let validated_vote_badge: ValidatedProof = vote_badge
            .validate_proof(
                ProofValidationMode::ValidateResourceAddress(self.vote_badge_address)
            )
            .expect("[Vote Info]: Invalid Proof provided.");

            // Retreives Vote Badge NFT data.
            let vote_badge_data = validated_vote_badge.non_fungible::<VoteBadge>().data();

            let proposal_id = vote_badge_data.proposal;
            let vote = vote_badge_data.vote;
            let vote_weight = vote_badge_data.vote_weight;

            assert_ne!(vote, vote_submission, "You have already voted {:?}.", 
                vote_submission
            );

            // Retrieves Proposal NFT Data again due to value being consumed.
            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);

            // The helper function that reallocates the LP Tokens from the respective vote vaults.
            let lp_tokens = self.reallocate_lp_token(&validated_vote_badge);

            // Evaluates the new vote and adds allocation based on matched criteria.
            // Data manipulation occurs for both Proposal NFT data and Vote Badge data.
            match vote_submission {
                // The scenario when the user recasts to a `Yes` vote. 
                Vote::Yes => {
                    self.voting_yes_vault.put(lp_tokens);

                    // Retrieves the Proposal NFT data to be changed.
                    let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    proposal_data.yes_counter += vote_weight;

                    info!("[Vote Recast]: You have voted 'Yes' for the proposal.");
                    info!("[Vote Recast]: The weight of your vote is {:?}.", vote_weight);

                    // Logic for changing the Vote Badge NFT
                    let mut vote_badge_data = validated_vote_badge.non_fungible::<VoteBadge>().data();
                    vote_badge_data.vote = Vote::Yes;

                    // Authorizes the updates for the Proposal NFT and the Vote Badge NFT.
                    self.nft_proposal_admin.authorize(||
                        validated_vote_badge.non_fungible().update_data(vote_badge_data)
                    );
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    // Retrieves the Proposal NFT data again since the previous value has been consumed.
                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;
    
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Vote Recast]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                    yes_counter, no_counter, no_with_veto_counter, abstain_counter);
                }
                // The scenario when the user recasts to a `No` vote. 
                Vote::No => {
                    self.voting_no_vault.put(lp_tokens);

                    let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    proposal_data.no_counter += vote_weight;

                    info!("[Vote Recast]: You have voted 'No' for the proposal.");
                    info!("[Vote Recast]: The weight of your vote is {:?}.", vote_weight);

                    // Logic for changing VoteBadge
                    let mut vote_badge_data = validated_vote_badge.non_fungible::<VoteBadge>().data();
                    vote_badge_data.vote = Vote::No;

                    self.nft_proposal_admin.authorize(||
                        validated_vote_badge.non_fungible().update_data(vote_badge_data)
                    );
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;
    
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Vote Recast]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                    yes_counter, no_counter, no_with_veto_counter, abstain_counter);
                }
                // The scenario when the user recasts to a `No with veto` vote. 
                Vote::NoWithVeto => {
                    self.voting_no_with_veto_vault.put(lp_tokens);

                    let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    proposal_data.no_with_veto_counter += vote_weight;

                    info!("[Vote Recast]: You have voted 'No' for the proposal.");
                    info!("[Vote Recast]: The weight of your vote is {:?}.", vote_weight);
      
                    // Logic for changing VoteBadge
                    let mut vote_badge_data = validated_vote_badge.non_fungible::<VoteBadge>().data();
                    vote_badge_data.vote = Vote::NoWithVeto;

                    self.nft_proposal_admin.authorize(||
                        validated_vote_badge.non_fungible().update_data(vote_badge_data)
                    );
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    let proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                    let yes_counter = proposal_data.yes_counter;
                    let no_counter = proposal_data.no_counter;
                    let no_with_veto_counter = proposal_data.no_with_veto_counter;
    
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Vote Recast]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                    yes_counter, no_counter, no_with_veto_counter, abstain_counter);
    
                    let abstain_counter = dec!("1.0") - (yes_counter + no_counter + no_with_veto_counter);
                    info!("[Vote Recast]: The current count for the vote is: Yes - {:?} | No - {:?} | No with veto - {:?} | Abstain - {:?}",
                    yes_counter, no_counter, no_with_veto_counter, abstain_counter);
                }
            }
        }

        /// This is the helper function for the `recast_vote` method.
        /// It essentially removes the LP Tokens from their respective vote vaults and changes the Proposal NFT data to reflect as such.
        /// This method performs a single check:
        /// * **Check 1:** Checks that there is enough LP Tokens to be redeemed.
        /// # Arguments:
        /// * `vote_badge` (&Proof) - The reference to the proof of the Vote Badge NFT.
        /// # Returns:
        /// * `Bucket` - The bucket that contains the LP Tokens.
        fn reallocate_lp_token(&mut self, vote_badge: &ValidatedProof) -> Bucket {
            
            // Retreives Vote Badge NFT data.
            let vote_badge_data = vote_badge.non_fungible::<VoteBadge>().data();

            let vote = vote_badge_data.vote;
            let proposal_id = vote_badge_data.proposal;
            let vote_weight = vote_badge_data.vote_weight;
            let lp_tokens_allocated = vote_badge_data.lp_tokens_allocated;

            // Retrieves Proposal NFT data.
            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);
            let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

            // Evaluates the current vote from Vote Badge NFT data and removes allocation based on matched criteria.
            // Data manipulation only happens with Proposal NFT data. 
            let lp_tokens: Bucket = match vote {
                // The scenario where the Vote Badge NFT has a `Yes` vote.
                Vote::Yes => {
                    assert!(
                        self.voting_yes_vault.amount() >= lp_tokens_allocated,
                        "[Withdraw]: Vault only has {:?} to withdraw.",
                        self.voting_yes_vault.amount()
                    );

                    let lp_tokens: Bucket = self.voting_yes_vault.take(lp_tokens_allocated);

                    proposal_data.yes_counter -= vote_weight;

                    // Authorizes the Proposal NFT data change.
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    info!("[Vote Recast]: Your allocations towards your 'Yes' cast has been decreased by {:?}.", 
                        lp_tokens_allocated);
                    info!("[Vote Recast]: Your weighted vote towards 'Yes' cast has been decreased by {:?}",
                        vote_weight);

                    lp_tokens
                }
                // The scenario where the Vote Badge NFT has a `No` vote.
                Vote::No => {
                    assert!(
                        self.voting_no_vault.amount() >= lp_tokens_allocated,
                        "[Withdraw]: Vault only has {:?} to withdraw.",
                        self.voting_no_vault.amount()
                    );

                    let lp_tokens: Bucket = self.voting_no_vault.take(lp_tokens_allocated);

                    proposal_data.no_counter -= vote_weight;

                    // Authorizes the Proposal NFT data change.
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    info!("[Vote Recast]: Your allocations towards your 'No' cast has been decreased by {:?}.", 
                        lp_tokens_allocated);
                    info!("[Vote Recast]: Your weighted vote towards 'No' cast has been decreased by {:?}",
                        vote_weight);

                    lp_tokens
                }
                // The scenario where the Vote Badge NFT has a `No with veto` vote.
                Vote::NoWithVeto => {
                    assert!(
                        self.voting_no_with_veto_vault.amount() >= lp_tokens_allocated,
                        "[Withdraw]: Vault only has {:?} to withdraw.",
                        self.voting_no_with_veto_vault.amount()
                    );

                    let lp_tokens: Bucket = self.voting_no_with_veto_vault.take(lp_tokens_allocated);

                    proposal_data.no_with_veto_counter -= vote_weight;

                    // Authorizes the Proposal NFT data change.
                    self.nft_proposal_admin.authorize(|| 
                        resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                    );

                    info!("[Vote Recast]: Your allocations towards your 'No with veto' cast has been decreased by {:?}.", 
                        lp_tokens_allocated);
                    info!("[Vote Recast]: Your weighted vote towards 'No with' cast has been decreased by {:?}",
                        vote_weight);

                    lp_tokens
                }
            };

            return lp_tokens
        }

        /// The method simply retrieves the LP Token amounts within each vote vaults.
        pub fn vault_amounts(&self) {

            info!("Yes: {:?}", self.voting_yes_vault.amount());
            info!("No: {:?}", self.voting_no_vault.amount());
            info!("No with veto: {:?}", self.voting_no_with_veto_vault.amount());
        }

        /// Checks if a finish condition is reached.
        /// This method has a few scenarios.
        /// * **Scenario 1:** - If there is a simple majority of `Yes` votes casted & the vote threshold is above 30% & the `No with veto`
        /// votes are less tha 33.4%.
        /// * **Resolution:** The proposal passes and the Proposal NFT is sent to the liquidity pool to change pool parameters. 
        /// * **Scenario 2:** - If there is a simple majority of `No` votes casted & the vote threshold is above 30%.
        /// There is no requirement for a `No with veto` condition, since there is already a simply majority within the participating votes.
        /// * **Resolution:** - The proposal fails and the Proposal NFT is sent to the liquidity pool to be burnt.
        /// * **Scenario 3:** - If the `No with veto` votes is equal to or greater than 33.4%.
        /// * **Resolution:** - The proposal fails and the Proposal NFT is sent to the liquidity pool to be burnt.
        /// * **Scenario 4:** - The time for the Voting Period stage has lapsed.
        /// * **Resolution:** - The proposal fails and the Proposal NFT is sent to the liquidity pool to be burnt.
        /// * **Scenario 5:** - No condition has reached.
        /// * **Resolution:** - Nothing happens. 
        /// This method does not perform any checks.
        /// This method does not request any arguments to be passed.
        /// This method does not return any assets. It evaluates the Proposal NFT in the Voting Period stage.
        pub fn resolve_proposal(&mut self) {

            // Retrieves vote calculations.
            let yes_vote = self.voting_yes_vault.amount();
            let no_vote = self.voting_no_vault.amount();
            let no_with_veto_vote = self.voting_no_with_veto_vault.amount();
            let participating_vote =  yes_vote + no_vote + no_with_veto_vote;
            let yes_counter = yes_vote / participating_vote;
            let no_counter = no_vote / participating_vote;
            let no_with_veto_counter = no_with_veto_vote / participating_vote;

            // Retrieves LiquidityPool component.
            let liquidity_pool: LiquidityPoolComponent = self.liquidity_pool_address.unwrap().into();

            // Retrieves LP Token resource address.
            let tracking_token_address = liquidity_pool.tracking_token_address();

            // Retrieves LP Token data.
            let tracking_tokens_manager: &ResourceManager = borrow_resource_manager!(tracking_token_address);

            // Calculates the vote threshold.
            let tracking_amount: Decimal = tracking_tokens_manager.total_supply();
            let vote_threshold = participating_vote / tracking_amount;

            // Scenario 1
            if yes_counter > no_counter && vote_threshold >= self.vote_quorom && no_with_veto_counter < self.no_with_veto_threshold {
                
                // Retrieves the Proposal NFT to be evaluated.
                let proposal_id = self.proposal_in_voting_period.pop().unwrap();

                // Retrieves the Proposal NFT from the proposal vault.
                let proposal = self.proposal_vault.take_non_fungible(&proposal_id);

                // Retrieves the Proposal NFT data.
                let mut proposal_data = proposal.non_fungible::<Proposal>().data();
                
                proposal_data.resolution = Resolution::Passed;
                
                // Authorizes the Proposal NFT data chnage.
                self.nft_proposal_admin.authorize(|| 
                    proposal.non_fungible().update_data(proposal_data)
                );

                // Retrieves the Proposal NFT data again since it the previous has been consumed.
                let proposal_data = proposal.non_fungible::<Proposal>().data();
                
                info!("[Proposal Resolution]: The proposal {:?} has passed!", proposal_id);
                info!("[Proposal Resolution]: 'Yes' count ({:?}) has the simple majority against the 'No' count ({:?}).",
                    yes_counter, no_counter
                );
                info!("[Proposal Resolution]: The pool paramaters has now changed to:");
                info!("[Proposal Resolution]: Token 1 weight: {:?} | Token 2 weight: {:?} | Fee to pool: {:?}",
                    proposal_data.token_1_weight, proposal_data.token_2_weight, proposal_data.fee_to_pool
                );
                
                // Sends to the liquidity pool.
                liquidity_pool.resolve_proposal(proposal);

            // Scenario 2
            } else if no_counter > yes_counter && vote_threshold >= self.vote_quorom {

                let proposal_id = self.proposal_in_voting_period.pop().unwrap();

                let proposal = self.proposal_vault.take_non_fungible(&proposal_id);

                let mut proposal_data = proposal.non_fungible::<Proposal>().data();
                
                proposal_data.resolution = Resolution::Failed;
                
                self.nft_proposal_admin.authorize(|| 
                    proposal.non_fungible().update_data(proposal_data)
                );
                
                info!("[Proposal Resolution]: The proposal {:?} has failed!", proposal_id);
                info!("[Proposal Resolution]: 'No' count ({:?}) has the simple majority against the 'Yes' count ({:?}).",
                    no_counter, yes_counter
                );

                liquidity_pool.resolve_proposal(proposal);

            // Scenario 3
            } else if no_with_veto_counter >= self.no_with_veto_threshold {

                let proposal_id = self.proposal_in_voting_period.pop().unwrap();

                let proposal = self.proposal_vault.take_non_fungible(&proposal_id);

                let mut proposal_data = proposal.non_fungible::<Proposal>().data();
                
                proposal_data.resolution = Resolution::Failed;
                
                self.nft_proposal_admin.authorize(|| 
                    proposal.non_fungible().update_data(proposal_data)
                );
                
                info!("[Proposal Resolution]: The proposal {:?} has failed!", proposal_id);
                info!("[Proposal Resolution]: 'No with veto' count ({:?}) has surpassed the threshold ({:?}).",
                    no_with_veto_counter, self.no_with_veto_threshold
                );

                liquidity_pool.resolve_proposal(proposal);

            // Scenario 4
            } else if Runtime::current_epoch() > self.voting_end_epoch {

                let proposal_id = self.proposal_in_voting_period.pop().unwrap();

                let proposal = self.proposal_vault.take_non_fungible(&proposal_id);

                let mut proposal_data = proposal.non_fungible::<Proposal>().data();
                
                proposal_data.resolution = Resolution::Failed;
                
                self.nft_proposal_admin.authorize(|| 
                    proposal.non_fungible().update_data(proposal_data)
                );
                
                liquidity_pool.resolve_proposal(proposal);

                info!("[Proposal Resolution]: The proposal {:?} has run out of time and failed!", proposal_id);

            // Scenario 5
            } else {

                info!("[Proposal Resolution]: No end condition reached");

            }
        }

        /// This method allows LPs to retrieve the LP Tokens.
        /// It removes the LPs votes if they have already voted in an on-going proposal in the Voting Period.
        /// This method performs a single check:
        /// * **Check 1:** - Checks that the Vote Badge NFT belongs to this protocol.
        /// # Arguments:
        /// * `vote_badge` (Bucket) - The bucket that contains the Vote Badge NFT.
        /// # Returns:
        /// * `Bucket` - The LP Tokens redeemed.
        pub fn retrieve_lp_tokens(&mut self, vote_badge: Bucket) -> Bucket {
            
            assert_eq!(vote_badge.resource_address(), self.vote_badge_address, "Incorrect badge");

            // Retrieves the Vote Badge NFT data.
            let vote_badge_data = vote_badge.non_fungible::<VoteBadge>().data();

            let amount = vote_badge_data.lp_tokens_allocated;
            let vote = vote_badge_data.vote;
            let proposal_id = vote_badge_data.proposal;
            let vote_weight = vote_badge_data.vote_weight;

            let resource_manager = borrow_resource_manager!(self.nft_proposal_address);

            // Matches the vote to its respective scenarios.
            match vote {
                // Scenario when the vote casted was a `Yes`.
                Vote::Yes => {
                    // Removes vote
                    if self.proposal_in_voting_period.contains(&proposal_id) {

                        let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                        proposal_data.yes_counter -= vote_weight;

                        self.nft_proposal_admin.authorize(|| 
                            resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                        );
                    }

                    // Redeems the LP Token.
                    let return_lp: Bucket = self.voting_yes_vault.take(amount);

                    // Burns the Vote Badge NFT.
                    self.nft_proposal_admin.authorize(|| vote_badge.burn());

                    info!("[Retrieve LP Tokens]: You have withdrawn {:?} LP Tokens", amount);
                    info!("[Retrieve LP Tokens]: Your `Yes` vote weight has been decreased by {:?}", vote_weight);
                    info!("[Retrieve LP Tokens]: Your Vote Badge NFT has been burnt.");

                    return_lp
                }
                // Scenario when the vote casted was a `No`.
                Vote::No => {
                    // Removes vote
                    if self.proposal_in_voting_period.contains(&proposal_id) {

                        let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                        proposal_data.no_counter -= vote_weight;

                        self.nft_proposal_admin.authorize(|| 
                            resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                        );
                    }

                    // Redeems the LP Token.
                    let return_lp: Bucket = self.voting_no_vault.take(amount);

                    // Burns the Vote Badge NFT.
                    self.nft_proposal_admin.authorize(|| vote_badge.burn());

                    info!("[Retrieve LP Tokens]: You have withdrawn {:?} LP Tokens", amount);
                    info!("[Retrieve LP Tokens]: Your `No` vote weight has been decreased by {:?}", vote_weight);
                    info!("[Retrieve LP Tokens]: Your Vote Badge NFT has been burnt.");

                    return_lp
                }
                // Scenario when the vote casted was a `No with veto`.
                Vote::NoWithVeto => {
                    // Removes vote
                    if self.proposal_in_voting_period.contains(&proposal_id) {

                        let mut proposal_data: Proposal = resource_manager.get_non_fungible_data(&proposal_id);

                        proposal_data.no_with_veto_counter -= vote_weight;

                        self.nft_proposal_admin.authorize(|| 
                            resource_manager.update_non_fungible_data(&proposal_id, proposal_data)
                        );
                    }

                    // Redeems the LP Token.
                    let return_lp: Bucket = self.voting_no_with_veto_vault.take(amount);

                    // Burns the Vote Badge NFT.
                    self.nft_proposal_admin.authorize(|| vote_badge.burn());

                    info!("[Retrieve LP Tokens]: You have withdrawn {:?} LP Tokens", amount);
                    info!("[Retrieve LP Tokens]: Your `No with veto` vote weight has been decreased by {:?}", vote_weight);
                    info!("[Retrieve LP Tokens]: Your Vote Badge NFT has been burnt.");

                    return_lp
                }
            }
        }

        /// This method submits the resource address of the Proposal NFT to the liquidity pool.
        pub fn push_nft_proposal_address(&self) {
            
            let liquidity_pool: LiquidityPoolComponent = self.liquidity_pool_address.unwrap().into();
            
            liquidity_pool.push_nft_proposal_address(self.nft_proposal_address);
        }
    }
}