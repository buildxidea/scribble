// src/demux.rs

//! Demux helpers for Symphonia.
//!
//! This module keeps container probing and packet iteration logic isolated from the
//! rest of the decode/transcode pipeline.
//!
//! Responsibilities:
//! - Probe a `MediaSource` and select a reasonable default audio track
//! - Provide a `next_packet` helper that treats IO errors as end-of-stream

use anyhow::{Context, Result, anyhow};
use symphonia::core::codecs::audio::CODEC_ID_NULL_AUDIO;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, Track};
use symphonia::core::io::{MediaSource, MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::packet::Packet;

/// Probe the container and pick a default audio track.
///
/// Track selection policy:
/// - choose the first track that looks decodable (codec != NULL)
/// - and has a known sample rate (required for resampling decisions downstream)
///
/// `hint_extension` can improve probe accuracy for ambiguous/unseekable inputs
/// (e.g. "mp4", "ts", "webm", "mkv", "ogg").
pub fn probe_source_and_pick_default_track(
    source: Box<dyn MediaSource>,
    hint_extension: Option<&str>,
) -> Result<(Box<dyn FormatReader>, Track)> {
    let mss_opts = MediaSourceStreamOptions {
        // Symphonia expects a power-of-two buffer > 32KiB for good probing behavior.
        buffer_len: 256 * 1024,
    };

    let mss = MediaSourceStream::new(source, mss_opts);

    let mut hint = Hint::new();
    if let Some(ext) = hint_extension {
        hint.with_extension(ext);
    }

    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();

    let format = symphonia::default::get_probe()
        .probe(&hint, mss, format_opts, metadata_opts)
        .map_err(|e| anyhow!(e))
        .context("failed to probe media stream")?;

    let track = format
        .tracks()
        .iter()
        .find(|track| {
            track
                .codec_params
                .as_ref()
                .and_then(|params| params.audio())
                .is_some_and(|params| {
                    params.codec != CODEC_ID_NULL_AUDIO && params.sample_rate.is_some()
                })
        })
        .cloned()
        .ok_or_else(|| anyhow!("no audio track found"))?;

    Ok((format, track))
}

/// Read the next packet, treating IO errors as "end of stream".
///
/// This makes decode loops simpler and streaming-friendly:
/// - `Ok(None)` means EOF or stream ended
/// - other errors are surfaced with context
pub fn next_packet(format: &mut Box<dyn FormatReader>) -> Result<Option<Packet>> {
    match format.next_packet() {
        Ok(packet) => Ok(packet),
        Err(SymphoniaError::IoError(_)) => Ok(None),
        Err(e) => Err(anyhow!(e)).context("failed reading packet"),
    }
}
