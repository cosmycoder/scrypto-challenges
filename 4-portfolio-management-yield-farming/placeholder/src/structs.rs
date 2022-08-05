use scrypto::prelude::*;

#[derive(NonFungibleData, Debug, Describe, Encode, Decode, TypeId, PartialEq)]
pub struct AssetManager {
    pub funds: HashMap<String, ComponentAddress>,
}

#[derive(Debug, Describe, Encode, Decode, TypeId, PartialEq)]
pub enum UserType {
    AssetManager,
    Investor,
}