//! MP3 → WAV for the native backend (ADR-043).
//!
//! quad-snd decodes only WAV and Ogg Vorbis natively, and panics on
//! anything else. MP3 clips are decoded here with symphonia into 16-bit
//! PCM wrapped in a WAV header, which quad-snd then loads as usual. On
//! the web the browser decodes MP3 itself, so this module is native only.

use std::io::Cursor;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// The container a clip's first bytes announce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Wav,
    Ogg,
    Mp3,
    Unknown,
}

pub fn sniff(bytes: &[u8]) -> Container {
    match bytes {
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'A',
            b'V',
            b'E',
            ..,
        ] => Container::Wav,
        [b'O', b'g', b'g', b'S', ..] => Container::Ogg,
        [b'I', b'D', b'3', ..] => Container::Mp3,
        // An MPEG audio frame sync: 11 set bits.
        [0xFF, b, ..] if b & 0xE0 == 0xE0 => Container::Mp3,
        _ => Container::Unknown,
    }
}

/// Decode an MP3 into a 16-bit PCM WAV file in memory.
pub fn to_wav(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            source,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("mp3 probe: {e}"))?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("mp3: no audio track")?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("mp3: no audio parameters")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| format!("mp3 decoder: {e}"))?;

    let mut pcm: Vec<i16> = Vec::new();
    let mut chunk: Vec<i16> = Vec::new();
    let mut spec: Option<(u32, usize)> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(e) => return Err(format!("mp3 read: {e}")),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                let s = buf.spec();
                spec.get_or_insert((s.rate(), s.channels().count()));
                chunk.resize(buf.samples_interleaved(), 0);
                buf.copy_to_slice_interleaved(&mut chunk);
                pcm.extend_from_slice(&chunk);
            }
            // A damaged frame: skip it, as players do.
            Err(Error::DecodeError(_)) => {}
            Err(e) => return Err(format!("mp3 decode: {e}")),
        }
    }
    let (rate, channels) = spec.ok_or("mp3: no audio frames")?;
    if !(1..=2).contains(&channels) {
        return Err(format!(
            "mp3: {channels} channels (only mono and stereo play)"
        ));
    }
    Ok(wav(&pcm, rate, channels as u16))
}

/// A canonical 44-byte-header PCM WAV.
pub fn wav(samples: &[i16], rate: u32, channels: u16) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let block_align = channels * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * block_align as u32).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_the_three_containers() {
        assert_eq!(sniff(&wav(&[0, 0], 44_100, 2)), Container::Wav);
        assert_eq!(sniff(b"OggS\0\x02"), Container::Ogg);
        assert_eq!(sniff(b"ID3\x04\0"), Container::Mp3);
        assert_eq!(sniff(&[0xFF, 0xFB, 0x90, 0x00]), Container::Mp3);
        assert_eq!(sniff(b"not audio"), Container::Unknown);
        assert_eq!(sniff(b""), Container::Unknown);
    }

    #[test]
    fn wav_header_describes_the_samples() {
        let w = wav(&[1, -1, 2, -2], 22_050, 2);
        assert_eq!(w.len(), 44 + 8);
        assert_eq!(&w[22..24], &2u16.to_le_bytes()); // channels
        assert_eq!(&w[24..28], &22_050u32.to_le_bytes()); // rate
        assert_eq!(&w[40..44], &8u32.to_le_bytes()); // data bytes
        assert_eq!(&w[44..46], &1i16.to_le_bytes());
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(to_wav(b"ID3 but nothing after").is_err());
    }
}

#[cfg(test)]
mod real_file {
    use super::*;

    /// jade-garden's opening track, the engine's first MP3 user.
    const TRACK: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../games/jade-garden/assets/sonican-modern-chinese-trip-asian-music-loop-499913.mp3"
    );

    #[test]
    fn a_real_mp3_decodes_to_a_playable_wav() {
        let Ok(bytes) = std::fs::read(TRACK) else {
            return; // the game is not checked out next to the engine
        };
        assert_eq!(sniff(&bytes), Container::Mp3);
        let start = std::time::Instant::now();
        let w = to_wav(&bytes).unwrap();
        eprintln!(
            "decoded {} KB → {} KB in {:?}",
            bytes.len() / 1024,
            w.len() / 1024,
            start.elapsed()
        );
        assert_eq!(sniff(&w), Container::Wav);
        let channels = u16::from_le_bytes([w[22], w[23]]);
        let rate = u32::from_le_bytes([w[24], w[25], w[26], w[27]]);
        assert!(channels == 1 || channels == 2);
        assert!((8_000..=96_000).contains(&rate));
        // Longer than a minute of audio.
        let seconds = (w.len() - 44) as f32 / (rate as f32 * channels as f32 * 2.0);
        assert!(seconds > 60.0, "{seconds} s");

        // quad-snd reads clips with audrey (and unwraps): it must accept
        // this WAV and see the same format.
        let mut reader = audrey::Reader::new(std::io::Cursor::new(&w)).expect("audrey reads it");
        let d = reader.description();
        assert_eq!(
            (d.channel_count(), d.sample_rate()),
            (channels as u32, rate)
        );
        assert!(reader.samples::<f32>().take(1000).all(|s| s.is_ok()));
    }
}
