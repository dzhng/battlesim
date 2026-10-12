//! The served MP3s (voice takes, music tracks), decoded packet by packet
//! from their bytes in memory, so a long track never sits decoded whole.
use std::io::Cursor;
use std::sync::Arc;

use kira::Frame;
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::{FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::units::Timestamp;

pub struct Mp3 {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
    /// The file's sample rate.
    pub rate: u32,
    samples: Vec<f32>,
}

impl Mp3 {
    pub fn open(bytes: Arc<[u8]>) -> Result<Self, String> {
        let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
        let reader = symphonia::default::get_probe()
            .probe(
                &Default::default(),
                source,
                Default::default(),
                Default::default(),
            )
            .map_err(|e| e.to_string())?;
        let track = reader
            .default_track(TrackType::Audio)
            .ok_or("no audio track")?;
        let Some(CodecParameters::Audio(params)) = track.codec_params.as_ref() else {
            return Err("no audio track".into());
        };
        let rate = params.sample_rate.ok_or("no sample rate")?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &Default::default())
            .map_err(|e| e.to_string())?;
        let track = track.id;
        Ok(Self {
            reader,
            decoder,
            track,
            rate,
            samples: Vec::new(),
        })
    }

    /// Appends the next packet's frames to `out` (mono as both sides);
    /// false at the end.
    pub fn next(&mut self, out: &mut Vec<Frame>) -> Result<bool, String> {
        let packet = loop {
            match self.reader.next_packet().map_err(|e| e.to_string())? {
                Some(p) if p.track_id == self.track => break p,
                Some(_) => {}
                None => return Ok(false),
            }
        };
        let buffer = self.decoder.decode(&packet).map_err(|e| e.to_string())?;
        let channels = buffer.num_planes();
        buffer.copy_to_vec_interleaved(&mut self.samples);
        match channels {
            1 => out.extend(self.samples.iter().map(|&s| Frame::from_mono(s))),
            2 => out.extend(
                self.samples
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&[l, r]| Frame::new(l, r)),
            ),
            n => return Err(format!("{n} channels")),
        }
        Ok(true)
    }

    /// Back to the start.
    pub fn rewind(&mut self) -> Result<(), String> {
        self.reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Timestamp {
                    ts: Timestamp::new(0),
                    track_id: self.track,
                },
            )
            .map_err(|e| e.to_string())?;
        self.decoder.reset();
        Ok(())
    }
}

/// All of `bytes`' frames, and its sample rate.
pub fn decode(bytes: Arc<[u8]>) -> Result<(u32, Vec<Frame>), String> {
    let mut mp3 = Mp3::open(bytes)?;
    let mut frames = Vec::new();
    while mp3.next(&mut frames)? {}
    Ok((mp3.rate, frames))
}
