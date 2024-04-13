use alloy_primitives::{address, Address, U256};
use alloy_sol_types::SolValue;
/// Replace config with arbitrum sepolia
use risc0_ethereum_view_call::{
    config::ETH_SEPOLIA_CHAIN_SPEC, ethereum::EthViewCallInput, ViewCall,
};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

sol! {
    interface SweatStake {
        function goalPerDayOf(address account, uint256 goalIndex) external view returns (uint256);
    }
}

fn main() {
   let input: EthViewCallInput = env::read();
   let contract: Address  = env::read();
   let account: Address = env::read();
   let goalIndex: U256 = env::read();
   let count: U256 = env::read();

   // Replace with arbitrum sepolia
   let view_call_env = input.into_env().with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);
   env::commit_slice(&view_call_env.block_commitment().abi_encode());

   let call = SweatStake::goalPerDayOfCall { account, goalIndex };
   let returns = ViewCall::new(call, contract).execute(view_call_env);
   assert!(returns._0 <= count);
}
