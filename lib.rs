#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod zeit_dao {
    use core::default;

    use ink::{prelude::vec::Vec, storage::Mapping};

    #[ink(storage)]
    pub struct ZeitDao {
        /// Stores a single `bool` value on the storage.
        value: bool,

        /* DAO Config */
        /// The members that have a vote within the DAO
        members: Vec<AccountId>,

        votes: Mapping<(AccountId, u16), bool>,
        /* Zeitgeist Components */
    }

    /// A smart contract that allows multiple users to come together to create a permissionless
    /// prediction market.
    impl ZeitDao {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(init_value: bool, _members: Vec<AccountId>) -> Self {
            Self {
                value: init_value,
                members: _members,
                votes: Mapping::default(),
            }
        }

        /// Returns all of the members within the DAO
        #[ink(message)]
        pub fn members(&self) -> Vec<AccountId> {
            self.members.iter().map(|x| x.clone()).collect()
        }

        #[ink(message)]
        pub fn is_member(&self) -> bool {
            self.members.contains(&Self::env().caller())
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
            let z = ZeitDao::new(false, x.clone());
            assert_eq!(z.members(), x);
        }

        #[ink::test]
        fn is_member_works() {
            let z1 = ZeitDao::new(
                false,
                vec![AccountId::from([0x01; 32]), AccountId::from([0x05; 32])]
            );
            assert_eq!(z1.is_member(), true);

            let z2: ZeitDao = ZeitDao::new(
                false,
                vec![AccountId::from([0x09; 32]), AccountId::from([0x05; 32])]
            );
            assert_eq!(z2.is_member(), false);
        }
    }
}
