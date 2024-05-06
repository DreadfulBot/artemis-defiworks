use std::future::Future;

use teloxide::RequestError;

pub enum ReportError {
    Telegram(RequestError),
}

pub trait Reporter {
    fn report(
        &self,
        target: i64,
        text: String,
    ) -> impl Future<Output = Result<(), ReportError>> + Send;
}
