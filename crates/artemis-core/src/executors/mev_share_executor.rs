use std::sync::Arc;

use crate::types::Executor;
use anyhow::Result;
use async_trait::async_trait;
use common::Reporter;
use ethers::signers::Signer;
use jsonrpsee::http_client::{
    transport::{self},
    HttpClientBuilder,
};
use mev_share::rpc::{FlashbotsSignerLayer, MevApiClient, SendBundleRequest};

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
impl<N: Reporter + Send + Sync> Executor<SendBundleRequest> for MevshareExecutor<N> {
    /// Send bundles to the matchmaker.
    async fn execute(&self, action: SendBundleRequest) -> Result<()> {
        let body = self.mev_share_client.send_bundle(action).await;
        let resp = match body {
            Ok(body) => format!("Bundle response: {body:?}"),
            Err(e) => format!("Bundle error: {e}"),
        };
        let _ = self.notifier.report(self.report_target, resp).await;
        Ok(())
    }
}
