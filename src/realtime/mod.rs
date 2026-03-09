pub mod openai;
pub mod protocol;

// Re-export for convenience
pub use openai::OpenAiRealtimeClient;
pub use protocol::{ClientEvent, ServerEvent};
