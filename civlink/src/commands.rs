// Copyright SM6WJM 2026

pub(crate) mod serve;
pub(crate) mod test_audio;
pub(crate) mod test_events;
pub(crate) mod user_add;
pub(crate) mod user_list;
pub(crate) mod user_remove;

pub use serve::run as serve;
pub use test_audio::run as test_audio;
pub use test_events::run as test_events;
pub use user_add::run as user_add;
pub use user_list::run as user_list;
pub use user_remove::run as user_remove;
