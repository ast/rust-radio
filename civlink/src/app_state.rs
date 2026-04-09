// Copyright SM6WJM 2026

use std::sync::Arc;

use crate::config::Config;
use crate::radio::Controller;
use crate::session::SessionStore;

pub struct AppState {
    pub config: Config,
    pub sessions: SessionStore,
    pub controller: Controller,
}

impl AppState {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            controller: Controller::new(),
        })
    }
}
