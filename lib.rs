#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use sp_runtime::MultiAddress;

// TODO: make it like a multisig and/or DAO, then just call self for DAO configuration instead
// of encoding all configuration possibilties into an enum

/// A smart contract meant to decentralize the permissionless creation of prediction markets between a
/// small (up to around 10) amount of users.
#[ink::contract]
mod zeit_dao {
    use crate::{AssetManagerCall, PredictionMarketsCall, RuntimeCall, ZeitgeistAsset};
    use ink::env::Error as EnvError;
    use ink::{prelude::vec::Vec, storage::Mapping};

    // region: Data Structures

    struct TransactionInput<'a>(&'a [u8]);
    impl<'a> scale::Encode for TransactionInput<'a> {
        fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
            dest.write(self.0);
        }
    }

    /// @dev Add additional actions that can be proposed
    #[derive(Debug, Clone, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]

    pub enum DAOAction {
        // Config

        // Runtime Actions
        RemarkWithEvent,
    }

    #[derive(Debug, Clone, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub struct StorableRuntimeAction {
        selector: DAOAction,
        data: Vec<u8>,
    }

    // endregion

    // region: Events & Errors

    #[ink(event)]
    pub struct TestEvent {
        sender: AccountId,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        executor: AccountId,
        id: u32,
        action: DAOAction,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ZeitDAOError {
        CallRuntimeFailed,
        OnlyMemberAllowed,
        OnlySelfAllowed,
        ProposalDoesNotExist,
        NotEnoughVotesApproved,
    }

    impl From<EnvError> for ZeitDAOError {
        fn from(e: EnvError) -> Self {
            match e {
                EnvError::CallRuntimeFailed => ZeitDAOError::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }
    // endregion

    #[ink(storage)]
    pub struct ZeitDao {
        /* DAO Config */
        /// The members that have a vote within the DAO.
        members: Vec<AccountId>,
        /// The votes for a specific account ID on a specific proposal version.
        votes: Mapping<(AccountId, u32), bool>,
        /// The number of aye votes needed before a proposal is accepted
        quorum: u32,

        /* Zeitgeist Components */
        proposals: Vec<StorableRuntimeAction>,
    }

    impl ZeitDao {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(_quorum: u32, _members: Vec<AccountId>) -> Self {
            if _quorum > _members.len() as u32 {
                panic!("Quorum must be less than or equal to length of members!");
            }
            Self {
                members: _members,
                votes: Mapping::default(),
                proposals: Vec::default(),
                quorum: _quorum,
            }
        }

        // region: Test Functions (remove later)

        #[ink(message)]
        pub fn test_event(&self) {
            self.env().emit_event(TestEvent {
                sender: self.env().caller(),
            });
        }

        #[ink(message)]
        pub fn test_asset_manager(&mut self) -> Result<(), ZeitDAOError> {
            // TODO: test to see if this works
            // Should send 1 ZTG to the user
            self.env()
                .call_runtime(&RuntimeCall::AssetManager(AssetManagerCall::Transfer {
                    dest: self.env().caller().into(),
                    currency_id: ZeitgeistAsset::Ztg,
                    amount: 10_000_000_000,
                }))
                .map_err(Into::into)
        }

        pub fn test_create_market(&mut self) -> Result<(), ZeitDAOError> {
            let sha3: [u8; 50] = [
                0x15, 0x30, 0x74, 0x0b, 0x1c, 0x25, 0x97, 0x6a, 0x79, 0xa1,
                0xd5, 0xe7, 0x5e, 0xfa, 0xb7, 0x70, 0x59, 0x61, 0x7c, 0xa2,
                0x26, 0x60, 0xe4, 0x3b, 0x2a, 0xa0, 0x5a, 0xe1, 0x4a, 0x59,
                0x94, 0xf9, 0xda, 0x2d, 0x9d, 0x71, 0xe4, 0x47, 0x37, 0x77,
                0xdd, 0x3d, 0x59, 0xac, 0x8c, 0x9a, 0x46, 0x1c, 0x7a, 0x68
            ];

            self.env()
                .call_runtime(&RuntimeCall::PredictionMarkets(
                    PredictionMarketsCall::CreateCpmmMarketAndDeployAssets {
                        base_asset: ZeitgeistAsset::Ztg,
                        creator_fee: 1000,
                        oracle: self.env().account_id(), // Puts self as oracle
                        period: crate::MarketPeriod::Block(core::ops::Range {
                            start: (self.env().block_number() + 1) as u64,
                            end: (self.env().block_number() + 150) as u64,
                        }),
                        deadlines: crate::Deadlines {
                            grace_period: 0,
                            oracle_duration: 28_800,
                            dispute_duration: 28_800,
                        },
                        metadata: crate::MultiHash::Sha3_384(sha3),
                        market_type: crate::MarketType::Categorical(2),
                        dispute_mechanism: crate::MarketDisputeMechanism::Authorized,
                        swap_fee: 10000,
                        amount: 1000,
                        weights: Vec::from([0, 1]),
                    },
                ))
                .map_err(Into::<ZeitDAOError>::into)?;
            Ok(())
        }

        // endregion

        /// Allows a member to create a new proposal for other members to vote on.
        /// @returns The proposal action.
        #[ink(message)]
        pub fn propose(&mut self, action: StorableRuntimeAction) -> Result<u32, ZeitDAOError> {
            self.only_member()?;
            self.proposals.push(action);
            Ok(self.proposals.len() as u32 - 1)
        }

        /// Allows a member to vote on a proposal.
        #[ink(message)]
        pub fn vote(&mut self, id: u32, direction: bool) -> Result<(), ZeitDAOError> {
            self.only_member()?;
            self.check_proposal_exists(id)?;
            self.votes.insert((self.env().caller(), id), &direction);
            Ok(())
        }

        // TODO: implement propose, vote, execute by stealing from the multisig
        // https://github.com/paritytech/ink-examples/blob/b5a5a554f85e9bd07d288ab319d14f15e6e509af/multisig/lib.rs

        // region: DAO Config Functions

        pub fn distribute(&mut self, balance: u128, target: AccountId) -> Result<(), ZeitDAOError> {
            self.only_self()?;
            self.env()
                .call_runtime(&RuntimeCall::AssetManager(AssetManagerCall::Transfer {
                    dest: target.into(),
                    currency_id: ZeitgeistAsset::Ztg,
                    amount: balance,
                }))
                .map_err(Into::into)
        }

        /*
        AddMember(AccountId),
        RemoveMember(AccountId),
        RuntimeCall(StorableRuntimeAction),
        Batch(Vec<DAOAction>),
        */

        // endregion

        // region: READ ONLY

        /// Returns all of the members within the DAO
        #[ink(message)]
        pub fn members(&self) -> Vec<AccountId> {
            self.members.iter().map(|x| x.clone()).collect()
        }

        /// True if the caller is a member, false if otherwise
        #[ink(message)]
        pub fn is_member(&self) -> bool {
            self.members.contains(&Self::env().caller())
        }

        /// Returns the information about a specific proposal. None if proposal does not exist.
        #[ink(message)]
        pub fn proposal(&self, id: u32) -> Option<StorableRuntimeAction> {
            if self.proposals.len() >= id as usize {
                None
            } else {
                Some(self.proposals[id as usize].clone())
            }
        }

        // endregion

        /* ================ PRIVATE / MODIFIERS ================ */

        fn only_member(&self) -> Result<(), ZeitDAOError> {
            if !self.is_member() {
                return Err(ZeitDAOError::OnlyMemberAllowed);
            }
            Ok(())
        }

        fn only_self(&self) -> Result<(), ZeitDAOError> {
            if self.env().caller() != self.env().account_id() {
                return Err(ZeitDAOError::OnlySelfAllowed);
            }
            Ok(())
        }

        fn check_proposal_exists(&self, id: u32) -> Result<(), ZeitDAOError> {
            if self.proposals.len() as u32 <= id {
                return Err(ZeitDAOError::ProposalDoesNotExist);
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
            let z = ZeitDao::new(1, x.clone());
            assert_eq!(z.members(), x);
        }

        #[ink::test]
        fn is_member_works() {
            let z1 = ZeitDao::new(
                2,
                vec![AccountId::from([0x01; 32]), AccountId::from([0x05; 32])],
            );
            assert_eq!(z1.is_member(), true);

            let z2: ZeitDao = ZeitDao::new(
                2,
                vec![AccountId::from([0x09; 32]), AccountId::from([0x05; 32])],
            );
            assert_eq!(z2.is_member(), false);
        }
    }
}

// region: Runtime Calls

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
#[derive(scale::Encode, scale::Decode)]
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
    #[codec(index = 40)]
    AssetManager(AssetManagerCall),
    #[codec(index = 57)]
    PredictionMarkets(PredictionMarketsCall),
}

#[derive(scale::Encode, scale::Decode)]
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

#[derive(scale::Encode, scale::Decode)]
enum AssetManagerCall {
    // https://github.com/open-web3-stack/open-runtime-module-library/blob/22a4f7b7d1066c1a138222f4546d527d32aa4047/currencies/src/lib.rs#L129-L131C19
    #[codec(index = 0)]
    Transfer {
        dest: MultiAddress<AccountId, ()>,
        currency_id: ZeitgeistAsset,
        #[codec(compact)]
        amount: u128,
    },
}

#[derive(scale::Encode, scale::Decode)]
enum PredictionMarketsCall {
    CreateCpmmMarketAndDeployAssets {
        base_asset: ZeitgeistAsset,
        // Used to be PerBill. I believe it's a u32 under the hood
        // https://paritytech.github.io/polkadot-sdk/master/src/sp_arithmetic/per_things.rs.html#1853
        creator_fee: u32,
        oracle: AccountId,
        // Used to be u64, MomentOf<T>, but unsure how to subsitute MomentOf<T>
        // Who needs timestamps anyways?
        period: MarketPeriod<u64, u64>,
        deadlines: Deadlines<u64>,
        metadata: MultiHash,
        market_type: MarketType,
        dispute_mechanism: MarketDisputeMechanism,
        #[codec(compact)]
        swap_fee: u128,
        #[codec(compact)]
        amount: u128,
        weights: Vec<u128>,
    },
}

// endregion

// region: Zeitgeist Types

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
enum ZeitgeistAsset {
    CategoricalOutcome, //(MI, CategoryIndex),
    ScalarOutcome,      //(MI, ScalarPosition),
    CombinatorialOutcome,
    PoolShare, //(SerdeWrapper<PoolId>),
    Ztg,       // default
    ForeignAsset(u32),
}

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
pub enum MarketDisputeMechanism {
    Authorized,
    Court,
    SimpleDisputes,
}

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
pub enum MarketType {
    /// A market with a number of categorical outcomes.
    Categorical(u16),
    /// A market with a range of potential outcomes.
    Scalar(core::ops::RangeInclusive<u128>),
}

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
pub enum MultiHash {
    Sha3_384([u8; 50]),
}

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
pub struct Deadlines<BN> {
    pub grace_period: BN,
    pub oracle_duration: BN,
    pub dispute_duration: BN,
}

#[derive(scale::Encode, scale::Decode, Clone, PartialEq)]
pub enum MarketPeriod<BN, M> {
    Block(core::ops::Range<BN>),
    Timestamp(core::ops::Range<M>),
}

// endregion
