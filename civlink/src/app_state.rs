// Copyright SM6WJM 2026

use std::sync::Arc;

use crate::audio::AudioCapture;
use crate::config::Config;
use crate::radio::RadioHandle;
use crate::session::SessionStore;

pub struct AppState {
    pub config: Config,
    pub sessions: SessionStore,
    pub audio: Option<AudioCapture>,
    pub radio: Option<RadioHandle>,
}

impl AppState {
    pub fn new(
        config: Config,
        audio: Option<AudioCapture>,
        radio: Option<RadioHandle>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            audio,
            radio,
        })
    }
}
