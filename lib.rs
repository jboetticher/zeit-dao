#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use sp_runtime::MultiAddress;

#[ink::contract]
mod zeit_dao {
    use ink::env::Error as EnvError;
    use ink::{
        prelude::{string::String, vec::Vec},
        storage::Mapping,
    };

    use crate::{RuntimeCall, SystemCall};

    /// @dev Add additional actions that can be proposed
    #[derive(Debug, Clone, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub enum ConfigAction {
        DistributeBalance(u32),
        AddMember(AccountId),
        RuntimeCall(RuntimeCall),
    }

    #[ink(storage)]
    pub struct ZeitDao {
        /* DAO Config */
        /// The members that have a vote within the DAO.
        members: Vec<AccountId>,
        /// The votes for a specific account ID on a specific proposal version.
        votes: Mapping<(AccountId, u32), bool>,

        /* Zeitgeist Components */
        messages: Vec<String>,
        proposals: Vec<ConfigAction>, // TODO: change into proposals, storing RuntimeCall | ConfigAction
    }

    #[ink(event)]
    pub struct TestEvent {
        sender: AccountId
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ZeitDAOError {
        CallRuntimeFailed,
        OnlyMemberAllowed,
    }

    impl From<EnvError> for ZeitDAOError {
        fn from(e: EnvError) -> Self {
            match e {
                EnvError::CallRuntimeFailed => ZeitDAOError::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }

    /// A smart contract that allows multiple users to come together to create a permissionless
    /// prediction market.
    impl ZeitDao {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(_members: Vec<AccountId>) -> Self {
            Self {
                members: _members,
                votes: Mapping::default(),
                messages: Vec::default(),
                proposals: Vec::default(),
            }
        }

        #[ink(message)]
        pub fn test_event(&self) {
            self.env().emit_event(TestEvent { sender: self.env().caller() });
        }

        #[ink(message)]
        pub fn test_asset_manager(&self) -> Result<(), ZeitDAOError> {
            let mut test_message = Vec::<u8>::new();
            test_message.push(2);

            self.env()
                .call_runtime(&RuntimeCall::System(SystemCall::RemarkWithEvent {
                    remark: test_message,
                }))
                .map_err(Into::into)

            // TODO: test to see if this works
            // Should send 0.3 ZTG to the user
            // self.env()
            //     .call_runtime(&RuntimeCall::AssetManager(AssetManagerCall::Transfer {
            //         dest: self.env().caller().into(),
            //         currency_id: ZeitgeistAsset::Ztg,
            //         amount: 3_000_000_000,
            //     }))
            //     .map_err(Into::into)
        }

        /// Allows a member to create a new proposal for other members to vote on.
        /// @returns The proposal action.
        #[ink(message)]
        pub fn propose(&mut self, action: ConfigAction) -> Result<u32, ZeitDAOError> {
            self.only_member()?;
            self.proposals.push(action);
            Ok(self.proposals.len() as u32 - 1)
        }

        /// Allows a member to vote on a proposal.
        /// @param id The id of the proposal to vote on.
        /// @param direction True if voting yes, false if voting no.
        /// @returns The proposal action.
        #[ink(message)]
        pub fn vote(&mut self, id: u32, direction: bool) -> Result<(), ZeitDAOError> {
            self.only_member()?;
            self.votes.insert((self.env().caller(), id), &direction);
            Ok(())
        }

        /* ==================== READ ONLY ==================== */

        /// Returns all of the members within the DAO
        #[ink(message)]
        pub fn members(&self) -> Vec<AccountId> {
            self.members.iter().map(|x| x.clone()).collect()
        }

        #[ink(message)]
        pub fn is_member(&self) -> bool {
            self.members.contains(&Self::env().caller())
        }

        fn only_member(&self) -> Result<(), ZeitDAOError> {
            if !self.is_member() {
                return Err(ZeitDAOError::OnlyMemberAllowed);
            }
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn initialze_with_members() {
            let x = vec![AccountId::from([0x01; 32]), AccountId::from([0x05; 32])];
            let z = ZeitDao::new(x.clone());
            assert_eq!(z.members(), x);
        }

        #[ink::test]
        fn is_member_works() {
            let z1 = ZeitDao::new(vec![
                AccountId::from([0x01; 32]),
                AccountId::from([0x05; 32]),
            ]);
            assert_eq!(z1.is_member(), true);

            let z2: ZeitDao = ZeitDao::new(vec![
                AccountId::from([0x09; 32]),
                AccountId::from([0x05; 32]),
            ]);
            assert_eq!(z2.is_member(), false);
        }
    }
}

// TODO: only these calls are allowed https://github.com/zeitgeistpm/zeitgeist/blob/3d9bbff91219bb324f047427224ee318061a6d43/runtime/battery-station/src/lib.rs#L121-L164

/// A part of the runtime dispatchable API.
///
/// For now, `ink!` doesn't provide any support for exposing the real `RuntimeCall` enum,
/// which fully describes the composed API of all the pallets present in runtime. Hence,
/// in order to use `call-runtime` functionality, we have to provide at least a partial
/// object, which correctly encodes the target extrinsic.
///
/// You can investigate the full `RuntimeCall` definition by either expanding
/// `construct_runtime!` macro application or by using secondary tools for reading chain
/// metadata, like `subxt`.
#[derive(Debug, Clone, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
)]
enum RuntimeCall {
    /// This index can be found by investigating runtime configuration. You can check the
    /// pallet order inside `construct_runtime!` block and read the position of your
    /// pallet (0-based).
    ///
    /// https://github.com/zeitgeistpm/zeitgeist/blob/3d9bbff91219bb324f047427224ee318061a6d43/runtime/common/src/lib.rs#L254-L363
    ///
    /// [See here for more.](https://substrate.stackexchange.com/questions/778/how-to-get-pallet-index-u8-of-a-pallet-in-runtime)
    #[codec(index = 0)]
    System(SystemCall),
    // #[codec(index = 40)]
    // AssetManager(AssetManagerCall),
}

#[derive(Debug, Clone, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
)]
enum SystemCall {
    /// This index can be found by investigating the pallet dispatchable API. In your
    /// pallet code, look for `#[pallet::call]` section and check
    /// `#[pallet::call_index(x)]` attribute of the call. If these attributes are
    /// missing, use source-code order (0-based).
    ///
    /// https://github.com/paritytech/substrate/blob/033d4e86cc7eff0066cd376b9375f815761d653c/frame/system/src/lib.rs#L512-L523
    #[codec(index = 7)]
    RemarkWithEvent { remark: Vec<u8> },
}

/*

#[derive(scale::Encode)]
enum AssetManagerCall {
    /// https://github.com/open-web3-stack/open-runtime-module-library/blob/22a4f7b7d1066c1a138222f4546d527d32aa4047/currencies/src/lib.rs#L129-L131C19
    #[codec(index = 0)]
    Transfer {
        dest: MultiAddress<AccountId, ()>,
        currency_id: ZeitgeistAsset,
        #[codec(compact)]
        amount: u128,
    },
}

#[derive(scale::Encode)]
pub enum ZeitgeistAsset {
    CategoricalOutcome, //(MI, CategoryIndex),
    ScalarOutcome,      //(MI, ScalarPosition),
    CombinatorialOutcome,
    PoolShare, //(SerdeWrapper<PoolId>),
    Ztg,       // default
    ForeignAsset(u32),
}

*/
