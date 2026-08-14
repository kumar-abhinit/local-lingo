pub mod cloud;
pub mod engine;
pub mod model_manager;
pub mod postprocess;
pub mod router;

pub use engine::{init_engine, validate_samples, AsrEngine};
pub use model_manager::{
    download_model, ensure_model_cached, list_available_models, model_path,
    model_path_for_tier, ModelInfo,
};
pub use postprocess::postprocess;
pub use router::{
    active_backend, asr_status, cloud_configured, ensure_transcription_ready, local_model_available,
    transcribe, AsrBackend, AsrStatus,
};
