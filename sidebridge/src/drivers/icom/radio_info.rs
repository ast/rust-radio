// Copyright SM6WJM 2026

use async_trait::async_trait;

use crate::traits::*;

use super::IcomRadio;

#[async_trait]
impl RadioInfo for IcomRadio {
    fn model(&self) -> &str {
        "IC-705"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            meter: true,
            scope: true,
            swr: true,
            memory_channels: false,
        }
    }
}
