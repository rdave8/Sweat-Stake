use alloy_primitives::{address, Address};
use alloy_sol_types::{sol, SolCall, SolInterface};
use anyhow::Result;
use apps::{BonsaiProver, TxSender};
use clap::Parser;
use erc20_counter_methods::IS_ENOUGH_ELF;
// Replace with arbitrum sepolia
use risc0_ethereum_view_call::{
    config::ETH_SEPOLIA_CHAIN_SPEC, ethereum::EthViewCallEnv, EvmHeader, ViewCall,
};
use risc0_zkvm::serde::to_vec;
use tracing_subscriber::EnvFilter;

sol! {
    interface SweatStake {
        function goalPerDayOf(address account, uint256 goalIndex) external view returns (uint256);
    }
}

sol! {
    interface SweatStake {
        function claim(bytes calldata journal, bytes32 postStateDigest, bytes calldata seal, uint256 goalIndex) public
    }
}

/// Arguments of the publisher CLI.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Ethereum chain ID
    #[clap(long)]
    chain_id: u64,

    /// Ethereum Node endpoint.
    #[clap(long, env)]
    eth_wallet_private_key: String,

    /// Ethereum Node endpoint.
    #[clap(long, env)]
    rpc_url: String,

    /// Counter's contract address on Ethereum
    #[clap(long)]
    contract: String,

    /// Account address to read the balance_of on Ethereum
    #[clap(long)]
    account: Address,
}

fn main() -> Result<()> {
    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    // parse the command line arguments
    let args = Args::parse();

    // Create a new `TxSender`.
    let tx_sender = TxSender::new(
        args.chain_id,
        &args.rpc_url,
        &args.eth_wallet_private_key,
        &args.contract,
    )?;

    // Create a view call environment from an RPC endpoint and a block number. If no block number is
    // provided, the latest block is used. The `with_chain_spec` method is used to specify the
    // chain configuration.
    let env =
        EthViewCallEnv::from_rpc(&args.rpc_url, None)?.with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);
    let number = env.header().number();

    // Function to call
    let account = args.account;
    let call = SweatStake::balanceOfCall { account };

    // Preflight the view call to construct the input that is required to execute the function in
    // the guest. It also returns the result of the call.
    let (view_call_input, returns) = ViewCall::new(call, CONTRACT).preflight(env)?;

    // Send an off-chain proof request to the Bonsai proving service.
    let input = InputBuilder::new()
        .write(view_call_input)
        .unwrap()
        .write(account)
        .unwrap()
        .bytes();
    let (journal, post_state_digest, seal) = BonsaiProver::prove(IS_ENOUGH_ELF, &input)?;

    let calldata = SweatStake::SweatStakeCalls::claim(SweatStake::claimCall {
        journal,
        post_state_digest,
        seal,
    })
    .abi_encode();

    // Send the calldata to Ethereum.
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(tx_sender.send(calldata))?;

    Ok(())
}

pub struct InputBuilder {
    input: Vec<u32>,
}

impl InputBuilder {
    pub fn new() -> Self {
        InputBuilder { input: Vec::new() }
    }

    pub fn write(mut self, input: impl serde::Serialize) -> Result<Self> {
        self.input.extend(to_vec(&input)?);
        Ok(self)
    }

    pub fn bytes(self) -> Vec<u8> {
        bytemuck::cast_slice(&self.input).to_vec()
    }
}