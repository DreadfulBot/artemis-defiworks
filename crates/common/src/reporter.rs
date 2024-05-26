use std::future::Future;

use async_zmq::errors::{RequestReplyError, SocketError};
use async_zmq::Error as ZmqError;
use teloxide::RequestError;

pub enum ReportError {
    Telegram(RequestError),
    ZMQ(ZmqError),
    Socket(SocketError),
    RequestReply(RequestReplyError),
}

pub trait Reporter<T> {
    fn report(&self, target: i64, data: T) -> impl Future<Output = Result<(), ReportError>> + Send;
}
