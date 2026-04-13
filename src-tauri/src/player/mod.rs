pub mod backend;
pub mod controller;
pub mod decode;
pub mod events;
pub mod state;

pub use controller::AudioPlayer;
pub use decode::decode_audio;
pub use state::PlayerState;