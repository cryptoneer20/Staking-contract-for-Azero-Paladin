#![cfg_attr(not(feature="std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod staking{
    use ink_storage::Mapping;
    use ink_storage::traits::{SpreadLayout, SpreadAllocate, StorageLayout, PackedLayout};
    use erc20::erc20::Erc20Ref;
    use ink_env::call;

    #[derive(Default, Debug, Eq, Clone,PartialEq, SpreadLayout, scale::Encode, scale::Decode, PackedLayout)]
    #[cfg_attr(feature="std", derive(StorageLayout, scale_info::TypeInfo))]
    pub struct Staker{
        stake_amount : Balance,
        reward_amount : Balance,
        last_time : Timestamp,
    }

    impl Staker{}

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Staking{
        staking_token : AccountId,
        apy : u32,
        total_reward_amount : Balance,
        total_stake_amount : Balance,
        staker_data : Mapping<AccountId, Staker>,
        interval : Timestamp,
    }

    #[ink(event)]
    pub struct StakeToken{
        #[ink(topic)]
        staker: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct UnstakeToken{
        #[ink(topic)]
        staker: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct ClaimRewards{
        #[ink(topic)]
        staker: AccountId,
        amount: Balance,
    }

    #[derive(Debug, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature="std", derive(scale_info::TypeInfo))]
    pub enum Error{
        InsufficientTokenBalance,
        InsufficientStakedBalance,
        InsufficientPoolTokenBalance,
        TokenTransferError,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    pub const SECONDOFYEAR : Timestamp = 365*24*60*60*1000;

    impl Staking{
        #[ink(constructor)]
        pub fn new(staking_token: AccountId, apy: u32) -> Self{
            ink_lang::utils::initialize_contract(|contract: &mut Self|{
                contract.staking_token = staking_token;//call::FromAccountId::from_account_id(staking_token);
                contract.apy = apy;
                // contract.interval = interval;
            })
        }

        #[ink(message)]
        pub fn staker_data_of(&self, staker: AccountId) -> Staker {
            self.staker_data_of_impl(&staker)
        }

        #[inline]
        fn staker_data_of_impl(&self, staker: &AccountId) -> Staker{
            self.staker_data.get(staker).unwrap_or_default()
        }

        #[ink(message)]
        pub fn apy(&self) -> u32 {
            self.apy
        }

        #[ink(message)]
        pub fn token_amount(&self, staker: AccountId) -> Balance{
            let staking_token : Erc20Ref = call::FromAccountId::from_account_id(self.staking_token);
            staking_token.balance_of(staker)
        }

        // #[ink(message)]
        // pub fn interval(&self) -> Timestamp {
        //  self.interval
        // }

        #[ink(message)]
        pub fn total_stake_amount(&self) -> Balance {
            self.total_stake_amount
        }

        #[ink(message)]
        pub fn total_reward_amount(&self) -> Balance {
            self.total_reward_amount
        }

        #[ink(message)]
        pub fn stake_token(&mut self, amount: Balance) -> Result<()>{
            let staker = self.env().caller();
            let mut staking_token : Erc20Ref = call::FromAccountId::from_account_id(self.staking_token);
            let staker_data = self.staker_data_of_impl(&staker);
            let staker_token_balance = staking_token.balance_of(staker);
            if staker_token_balance < amount{
                return Err(Error::InsufficientTokenBalance);
            }

            let new_reward = staker_data.stake_amount * (self.env().block_timestamp() - staker_data.last_time) as u128 * self.apy as u128 / 10000 / SECONDOFYEAR as u128;

             if staking_token.transfer_from(staker, self.env().account_id(), amount) != Ok(()){
                return Err(Error::TokenTransferError);
             }
            self.staker_data.insert(&staker, &Staker{
                stake_amount: staker_data.stake_amount + amount,
                reward_amount: staker_data.reward_amount + new_reward, //need to be updated
                last_time: self.env().block_timestamp()
            });
            self.total_stake_amount = self.total_stake_amount + amount;
            // self.env().emit_event(StakeToken{staker: staker, amount});
            Ok(())
        }

        #[ink(message)]
        pub fn unstake_token(&mut self, amount: Balance) -> Result<()>{
            let staker = self.env().caller();
            let mut staking_token : Erc20Ref = call::FromAccountId::from_account_id(self.staking_token);
            let staker_data = self.staker_data_of_impl(&staker);
            if staker_data.stake_amount < amount{
                return Err(Error::InsufficientStakedBalance);
            }
            let pool_token_amount = staking_token.balance_of(self.env().account_id());
            if pool_token_amount < amount{
                return Err(Error::InsufficientPoolTokenBalance);
            }

            let new_reward = staker_data.stake_amount * (self.env().block_timestamp() - staker_data.last_time) as u128 * self.apy as u128 / 10000 / SECONDOFYEAR as u128;

            if staking_token.transfer(staker, amount) != Ok(()){
                return Err(Error::TokenTransferError);
            }
            self.staker_data.insert(&staker, &Staker{
                stake_amount: staker_data.stake_amount - amount,
                reward_amount: staker_data.reward_amount + new_reward,
                last_time: self.env().block_timestamp()
            });
            self.total_stake_amount = self.total_stake_amount - amount;
            // self.env().emit_event(UnstakeToken{staker: staker, amount});
            Ok(())
        }

        #[ink(message)]
        pub fn claim_rewards(&mut self) -> Result<()>{
            let staker = self.env().caller();
            let mut staking_token : Erc20Ref = call::FromAccountId::from_account_id(self.staking_token);
            let staker_data =self.staker_data_of_impl(&staker);

            let new_reward = staker_data.stake_amount * (self.env().block_timestamp() - staker_data.last_time) as u128 * self.apy as u128 / 10000 / SECONDOFYEAR as u128;

            let reward_amount = staker_data.reward_amount + new_reward;
            let pool_token_amount = staking_token.balance_of(self.env().account_id());
            if pool_token_amount < reward_amount{
                return Err(Error::InsufficientPoolTokenBalance);
            }
            if staking_token.transfer(staker, reward_amount) != Ok(()){
                return Err(Error::TokenTransferError);
            }
            self.staker_data.insert(&staker, &Staker{
                stake_amount: staker_data.stake_amount,
                reward_amount: 0,
                last_time: self.env().block_timestamp()
            });
            self.total_reward_amount = self.total_reward_amount + reward_amount;
            // self.env().emit_event(ClaimRewards{staker: staker, amount: reward_amount});
            Ok(())
        }
    }
}