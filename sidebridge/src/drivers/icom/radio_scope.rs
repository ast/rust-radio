// Copyright SM6WJM 2026

use async_trait::async_trait;
use tokio_stream::wrappers::BroadcastStream;

use crate::traits::*;

use super::{IcomRadio, REQUEST_TIMEOUT};

#[async_trait]
impl RadioScope for IcomRadio {
    async fn scope_data(&self) -> Result<ScopeFrame> {
        let mut rx = self.scope_tx.subscribe();
        match tokio::time::timeout(REQUEST_TIMEOUT, rx.recv()).await {
            Ok(Ok(frame)) => Ok(frame),
            Ok(Err(_)) => Err(RadioError::Protocol("scope channel closed".into())),
            Err(_) => Err(RadioError::Timeout("no scope data received".into())),
        }
    }

    fn scope_stream(&self) -> Box<dyn tokio_stream::Stream<Item = ScopeFrame> + Send + Unpin> {
        let rx = self.scope_tx.subscribe();
        Box::new(tokio_stream::StreamExt::filter_map(
            BroadcastStream::new(rx),
            |r: std::result::Result<ScopeFrame, _>| r.ok(),
        ))
    }
}
