// Copyright SM6WJM 2026

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::broadcast;
use webrtc::api::media_engine::{MediaEngine, MIME_TYPE_OPUS};
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc_media::Sample;

use crate::audio::AudioFrame;
use crate::Result;
use crate::error::CivlinkError;

/// Creates a new WebRTC peer connection with an audio track that streams
/// Opus-encoded audio from the broadcast channel to the remote browser.
pub async fn create_peer_connection(
    audio_rx: broadcast::Receiver<AudioFrame>,
) -> Result<(Arc<RTCPeerConnection>, Arc<TrackLocalStaticSample>)> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| CivlinkError::WebRtc(format!("failed to register codecs: {e}")))?;

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(
        api.new_peer_connection(config)
            .await
            .map_err(|e| CivlinkError::WebRtc(format!("failed to create peer connection: {e}")))?,
    );

    // Create local audio track (Opus)
    let audio_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_OPUS.to_owned(),
            clock_rate: 48000,
            channels: 2,
            sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
            rtcp_feedback: vec![],
        },
        "audio".to_owned(),
        "civlink-audio".to_owned(),
    ));

    peer_connection
        .add_track(audio_track.clone())
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to add audio track: {e}")))?;

    tracing::info!("WebRTC peer connection created with audio track");

    // Spawn a task that reads from the broadcast channel and writes to the track
    let track = audio_track.clone();
    tokio::spawn(async move {
        pump_audio(audio_rx, track).await;
    });

    Ok((peer_connection, audio_track))
}

/// Reads Opus frames from the broadcast channel and writes them to the WebRTC track.
async fn pump_audio(
    mut rx: broadcast::Receiver<AudioFrame>,
    track: Arc<TrackLocalStaticSample>,
) {
    tracing::debug!("audio pump started");
    loop {
        match rx.recv().await {
            Ok(frame) => {
                let sample = Sample {
                    data: frame.data,
                    timestamp: SystemTime::now(),
                    duration: Duration::from_millis(frame.duration_ms as u64),
                    packet_timestamp: 0,
                    prev_dropped_packets: 0,
                    prev_padding_packets: 0,
                };
                if let Err(e) = track.write_sample(&sample).await {
                    tracing::error!("failed to write audio sample to track: {e}");
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("audio pump lagged, skipped {n} frames");
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::info!("audio broadcast channel closed, stopping pump");
                break;
            }
        }
    }
}
