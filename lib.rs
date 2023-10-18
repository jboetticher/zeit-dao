#![cfg_attr(not(feature = "std"), no_std, no_main)]

use ink::prelude::vec::Vec;
use ink::primitives::AccountId;
use sp_runtime::MultiAddress;

/// A smart contract meant to decentralize the permissionless creation of prediction markets between a
/// small (up to around 10) amount of users.
#[ink::contract]
mod zeit_dao {
    use crate::{AssetManagerCall, RuntimeCall, SystemCall, TestRuntimeCall, ZeitgeistAsset};
    use ink::env::Error as EnvError;
    use ink::{prelude::vec::Vec, storage::Mapping};

    // region: Data Structures

    /// @dev Add additional actions that can be proposed
    #[derive(Debug, Clone, scale::Decode, scale::Encode)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
    )]
    pub enum DAOAction {
        DistributeBalance(u32),
        AddMember(AccountId),
        RemoveMember(AccountId),
        // RuntimeCall(RuntimeCall),
        Batch(Vec<DAOAction>),
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
        proposals: Vec<DAOAction>,
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
        pub fn test_asset_manager(&self) -> Result<(), ZeitDAOError> {
            // TODO: test to see if this works
            // Should send 1 ZTG to the user
            self.env()
                .call_runtime(&TestRuntimeCall::AssetManager(AssetManagerCall::Transfer {
                    dest: self.env().caller().into(),
                    currency_id: ZeitgeistAsset::Ztg,
                    amount: 10_000_000_000,
                }))
                .map_err(Into::into)
        }

        // endregion

        /// Allows a member to create a new proposal for other members to vote on.
        /// @returns The proposal action.
        #[ink(message)]
        pub fn propose(&mut self, action: DAOAction) -> Result<u32, ZeitDAOError> {
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

        #[ink(message)]
        pub fn execute(&mut self, id: u32) -> Result<(), ZeitDAOError> {
            self.check_proposal_exists(id)?;

            // Counts the total amount of votes
            // TODO: optimize for large scale DAOs
            let vote_total = self.members.iter().fold(0, |acc, m| {
                if self.votes.get((m, id)).unwrap_or(false) {
                    acc + 1
                } else {
                    acc
                }
            });
            if vote_total < self.quorum {
                return Err(ZeitDAOError::NotEnoughVotesApproved);
            }

            let action = &self.proposals[id as usize];
            match action {
                DAOAction::DistributeBalance(amount) => {
                    // Will likely cause a revert if the amount is too large
                    let amount_per_member = amount / self.members.len() as u32;
                    self.env().call_runtime(&TestRuntimeCall::AssetManager(
                        AssetManagerCall::Transfer {
                            dest: self.env().caller().into(),
                            currency_id: ZeitgeistAsset::Ztg,
                            amount: amount_per_member as u128,
                        },
                    ))?;
                }
                DAOAction::AddMember(new_member) => self.members.push(*new_member),
                DAOAction::RemoveMember(member) => self.members.retain(|m| m != member),
                // DAOAction::RuntimeCall(_) => todo!(),
                _ => todo!(),
            };

            // Emit event
            self.env().emit_event(ProposalExecuted {
                executor: self.env().caller(),
                id,
                action: action.clone(),
            });

            Ok(())
        }

        /* ==================== READ ONLY ==================== */

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
        pub fn proposal(&self, id: u32) -> Option<DAOAction> {
            if self.proposals.len() >= id as usize {
                None
            } else {
                Some(self.proposals[id as usize].clone())
            }
        }

        /* ================ PRIVATE / MODIFIERS ================ */

        fn only_member(&self) -> Result<(), ZeitDAOError> {
            if !self.is_member() {
                return Err(ZeitDAOError::OnlyMemberAllowed);
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

/* THE FOLLOWING FEATURES ARE STILL IN DEVELOPMENT AND ARE POO POO */

#[derive(Debug, Clone, scale::Decode, scale::Encode)]
enum TestRuntimeCall {
    #[codec(index = 40)]
    AssetManager(AssetManagerCall),
}

#[derive(Debug, Clone, scale::Decode, scale::Encode)]
// #[cfg_attr(
//     feature = "std",
//     derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
// )]
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

#[derive(Debug, Clone, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(ink::storage::traits::StorageLayout, scale_info::TypeInfo)
)]
pub enum ZeitgeistAsset {
    CategoricalOutcome, //(MI, CategoryIndex),
    ScalarOutcome,      //(MI, ScalarPosition),
    CombinatorialOutcome,
    PoolShare, //(SerdeWrapper<PoolId>),
    Ztg,       // default
    ForeignAsset(u32),
}
