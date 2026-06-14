use std::sync::Once;

/// Ensure whisper logging is configured exactly once for the lifetime of the process.
///
/// Routes whisper.cpp and GGML logs through whisper-rs's logging hooks. scribble enables
/// neither the `log_backend` nor `tracing_backend` feature on whisper-rs, so the hooks
/// compile down to no-ops and the logs are silenced. This is the same observable behavior
/// as the previous no-op `whisper_log_set` callback: whisper.cpp's `whisper_log_set`
/// internally forwards to `ggml_log_set`, so both the old and new paths silence the
/// whisper *and* GGML log streams.
pub fn init_whisper_logging() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        whisper_rs::install_logging_hooks();
    });
}
