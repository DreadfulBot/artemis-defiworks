use common::Reporter;
use serde::Serialize;
use serde_json::json;
use zmq::Context;

/// This reporter sends the message to any external service through zeromq
pub struct ExternalReporter {
    port: String,
    context: Context,
}

impl ExternalReporter {
    pub fn new(port: String) -> Self {
        let context = Context::new();
        Self { port, context }
    }
}

impl<T: Serialize + Send + Sync> Reporter<T> for ExternalReporter {
    async fn report(&self, target: i64, data: T) -> Result<(), common::ReportError> {
        let msg = json!({
            "target": target,
            "data": data
        })
        .to_string();
        let requester = self.context.socket(zmq::PUSH).unwrap();
        let port = &self.port;
        requester
            .connect(&format!("tcp://localhost:{port}"))
            .map_err(common::ReportError::ZMQ)?;
        requester.send(&msg, 0).map_err(common::ReportError::ZMQ)?;
        Ok(())
    }
}
