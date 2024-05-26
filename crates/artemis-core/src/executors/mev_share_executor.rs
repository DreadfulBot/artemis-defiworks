use std::sync::Arc;

use crate::types::Executor;
use anyhow::Result;
use async_trait::async_trait;
use common::Reporter;
use ethers::signers::Signer;
use ethers::types::TxHash;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{transport, HttpClient, HttpClientBuilder},
};
use mev_share::rpc::{
    FlashbotsSigner, FlashbotsSignerLayer, MevApiClient, SendBundleRequest, SendBundleResponse,
};
use serde::Serialize;
use tokio::sync::mpsc::{UnboundedReceiver as Receiver, UnboundedSender as Sender};
use tower::util::MapErr;
use tracing::{error, info};

/// An executor that sends bundles to the MEV-share Matchmaker.
pub struct MevshareExecutor<N> {
    mev_share_client: Box<dyn MevApiClient + Send + Sync>,
    notifier: Arc<N>,
    report_target: i64,
}

impl<N> MevshareExecutor<N> {
    pub fn new(
        signer: impl Signer + Clone + 'static,
        notifier: Arc<N>,
        report_target: i64,
    ) -> Self {
        // Set up flashbots-style auth middleware
        let http = HttpClientBuilder::default()
            .set_middleware(
                tower::ServiceBuilder::new()
                    .map_err(transport::Error::Http)
                    .layer(FlashbotsSignerLayer::new(signer)),
            )
            .build("https://relay.flashbots.net:443")
            .expect("failed to build HTTP client");
        Self {
            mev_share_client: Box::new(http),
            notifier,
            report_target,
        }
    }
}

#[async_trait]
impl<N: Reporter<BundleData> + Send + Sync> Executor<SendBundleRequest> for MevshareExecutor<N> {
    /// Send bundles to the matchmaker.
    async fn execute(&self, action: SendBundleRequest) -> Result<()> {
        let block = action.inclusion.block.as_u64();
        let body = self.mev_share_client.send_bundle(action).await;
        match body {
            Ok(body) => {
                info!("Bundle response: {body:?}");
                let report_data = BundleData { hash: body, block };
                let _ = self.notifier.report(self.report_target, report_data).await;
            }
            Err(e) => {
                error!("Bundle error: {e}")
            }
        };
        Ok(())
    }
}

#[derive(Serialize)]
struct BundleData {
    hash: SendBundleResponse,
    block: u64,
}
