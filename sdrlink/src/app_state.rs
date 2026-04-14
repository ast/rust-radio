use std::sync::Arc;

use crate::config::Config;
use crate::sdr::SdrHandle;
use crate::session::SessionStore;

pub struct AppState {
    pub config: Config,
    pub sessions: SessionStore,
    pub sdr: SdrHandle,
}

impl AppState {
    pub fn new(config: Config, sdr: SdrHandle) -> Arc<Self> {
        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            sdr,
        })
    }
}
