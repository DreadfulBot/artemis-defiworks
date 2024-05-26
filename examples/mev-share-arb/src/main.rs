use std::sync::Arc;

use anyhow::Result;
use artemis_core::{
    collectors::mevshare_collector::MevShareCollector,
    engine::Engine,
    executors::mev_share_executor::MevshareExecutor,
    types::{CollectorMap, ExecutorMap},
};
use clap::Parser;
use ethers::{
    prelude::MiddlewareBuilder,
    providers::{Provider, Ws},
    signers::{LocalWallet, Signer},
    types::Address,
};
use external::ExternalReporter;
use mev_share_uni_arb::{
    strategy::MevShareUniArb,
    types::{Action, Event},
};
use tracing::{info, Level};
use tracing_subscriber::{filter, prelude::*};

/// CLI Options.
#[derive(Parser, Debug)]
pub struct Args {
    /// Ethereum node WS endpoint.
    #[arg(long)]
    pub wss: String,
    /// Private key for sending txs.
    #[arg(long)]
    pub private_key: String,
    /// MEV share signer
    #[arg(long)]
    pub flashbots_signer: String,
    /// Path to file with pools
    #[arg(long)]
    pub pool_json_path: String,
    /// Path to file with pools
    #[arg(long)]
    pub log_path: String,
    /// Address of the arb contract.
    #[arg(long)]
    pub arb_contract_address: Address,
    #[arg(long)]
    pub telegram_chat: i64,
    #[arg(long)]
    pub external_port: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let log_path = args.log_path;
    // Set up tracing and parse args.
    let file_appender =
        tracing_appender::rolling::hourly(format!("{log_path}/mev-share-arbitrage"), "test.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = filter::Targets::new()
        .with_target("mev_share_uni_arb", Level::INFO)
        .with_target("artemis_core", Level::INFO);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(filter)
        .init();

    let external_reporter = Arc::new(ExternalReporter::new(args.external_port));

    //  Set up providers and signers.
    let ws = Ws::connect(args.wss).await?;
    let provider = Provider::new(ws);

    let wallet: LocalWallet = args.private_key.parse().unwrap();
    let address = wallet.address();

    let provider = Arc::new(provider.nonce_manager(address).with_signer(wallet.clone()));
    let fb_signer: LocalWallet = args.flashbots_signer.parse().unwrap();

    // Set up engine.
    let mut engine: Engine<Event, Action> = Engine::default();

    // Set up collector.
    let mevshare_collector = Box::new(MevShareCollector::new(String::from(
        "https://mev-share.flashbots.net",
    )));
    let mevshare_collector = CollectorMap::new(mevshare_collector, Event::MEVShareEvent);
    engine.add_collector(Box::new(mevshare_collector));

    // Set up strategy.
    let strategy = MevShareUniArb::new(
        Arc::new(provider.clone()),
        wallet,
        args.arb_contract_address,
        args.pool_json_path,
    );
    engine.add_strategy(Box::new(strategy));

    // Set up executor.
    let mev_share_executor = Box::new(MevshareExecutor::new(
        fb_signer,
        external_reporter.clone(),
        args.telegram_chat,
    ));
    let mev_share_executor = ExecutorMap::new(mev_share_executor, |action| match action {
        Action::SubmitBundle(bundle) => Some(bundle),
    });
    engine.add_executor(Box::new(mev_share_executor));

    // Start engine.
    if let Ok(mut set) = engine.run().await {
        while let Some(res) = set.join_next().await {
            info!("res: {:?}", res);
        }
    }

    Ok(())
}
