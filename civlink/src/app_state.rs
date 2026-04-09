// Copyright SM6WJM 2026

use std::sync::Arc;

use crate::audio::AudioCapture;
use crate::config::Config;
use crate::radio::Controller;
use crate::session::SessionStore;

pub struct AppState {
    pub config: Config,
    pub sessions: SessionStore,
    pub controller: Controller,
    pub audio: Option<AudioCapture>,
}

impl AppState {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            controller: Controller::new(),
            audio: None,
        })
    }

    pub fn with_audio(config: Config, audio: AudioCapture) -> Arc<Self> {
        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            controller: Controller::new(),
            audio: Some(audio),
        })
    }
}
