use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::codecs::{CODEC_TYPE_AAC, CODEC_TYPE_OPUS, CodecType};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::units::TimeBase;

const OPUS_RATE: u64 = 48_000;
const AAC_FRAME_SAMPLES: u64 = 1024;

pub fn opus_packet_samples(packet: &[u8]) -> u64 {
    let Some(&toc) = packet.first() else {
        return 0;
    };
    let config = toc >> 3;
    let frame_tenths_ms: u64 = match config {
        0..=11 => [100, 200, 400, 600][(config % 4) as usize],
        12..=15 => [100, 200][(config % 2) as usize],
        _ => [25, 50, 100, 200][(config % 4) as usize],
    };
    let frames: u64 = match toc & 0x03 {
        0 => 1,
        1 | 2 => 2,
        _ => packet.get(1).map(|b| (b & 0x3F) as u64).unwrap_or(0),
    };
    frame_tenths_ms * frames * OPUS_RATE / 10_000
}

fn codec_samples(codec: CodecType, packet: &[u8]) -> Option<u64> {
    if codec == CODEC_TYPE_OPUS {
        Some(opus_packet_samples(packet))
    } else if codec == CODEC_TYPE_AAC {
        Some(AAC_FRAME_SAMPLES)
    } else {
        None
    }
}

pub struct VoiceSpool {
    path: PathBuf,
    writer: Mutex<Option<BufWriter<File>>>,
}

impl VoiceSpool {
    pub fn create() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!("rift-voice-{}", uuid::Uuid::new_v4()));
        let file = File::create(&path)?;
        Ok(Self {
            path,
            writer: Mutex::new(Some(BufWriter::new(file))),
        })
    }

    pub fn write(&self, chunk: &[u8]) -> std::io::Result<()> {
        let mut guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some(writer) => writer.write_all(chunk),
            None => Err(std::io::Error::other("voice spool is closed")),
        }
    }

    pub fn probe_video_note(&self, content_type: &str) -> Option<super::video_probe::VideoProbe> {
        let writer = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        writer.into_inner().ok()?.sync_all().ok()?;
        let file = File::open(&self.path).ok()?;
        super::video_probe::probe_video_note(Box::new(file), content_type)
    }

    pub fn probe_duration_ms(&self) -> Option<i64> {
        let writer = self.writer.lock().unwrap_or_else(|e| e.into_inner()).take()?;
        writer.into_inner().ok()?.sync_all().ok()?;
        let file = File::open(&self.path).ok()?;
        probe_source(Box::new(file))
    }
}

impl Drop for VoiceSpool {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn probe_voice_duration_ms(bytes: Vec<u8>) -> Option<i64> {
    probe_source(Box::new(Cursor::new(bytes)))
}

fn probe_source(media: Box<dyn MediaSource>) -> Option<i64> {
    let source = MediaSourceStream::new(media, Default::default());
    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), source, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;
    let track = match format.tracks() {
        [only] => only.clone(),
        _ => return None,
    };
    if track.codec_params.codec != CODEC_TYPE_OPUS && track.codec_params.codec != CODEC_TYPE_AAC {
        return None;
    }
    let time_base = track
        .codec_params
        .time_base
        .or_else(|| track.codec_params.sample_rate.map(|rate| TimeBase::new(1, rate)))?;
    let codec = track.codec_params.codec;
    let codec_rate = if codec == CODEC_TYPE_OPUS {
        Some(OPUS_RATE)
    } else {
        track.codec_params.sample_rate.map(u64::from)
    };
    let mut end: u64 = 0;
    let mut samples: u64 = 0;
    let mut packets = 0usize;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return None,
        };
        if packet.track_id() != track.id {
            return None;
        }
        packets += 1;
        end = end.max(packet.ts.saturating_add(packet.dur));
        if let Some(n) = codec_samples(codec, &packet.data) {
            samples = samples.saturating_add(n);
        }
    }
    if packets == 0 {
        return None;
    }
    let time = time_base.calc_time(end);
    let container_ms = (time.seconds as f64 * 1000.0 + time.frac * 1000.0).round() as i64;
    let codec_ms = match codec_rate {
        Some(rate) if rate > 0 => (samples.saturating_mul(1000) / rate) as i64,
        _ => 0,
    };
    Some(container_ms.max(codec_ms))
}

#[cfg(test)]
mod tests {
    use super::probe_voice_duration_ms;

    #[test]
    fn reads_the_length_of_a_browser_recording() {
        let bytes = include_bytes!("fixtures/voice-4s.webm").to_vec();
        let ms = probe_voice_duration_ms(bytes).expect("duration");
        assert!((3500..=4300).contains(&ms), "{ms}");
    }

    #[test]
    fn counts_opus_frames_from_the_packet_header() {
        use super::opus_packet_samples;
        assert_eq!(opus_packet_samples(&[0b1111_1000]), 960);
        assert_eq!(opus_packet_samples(&[0b0001_1000]), 2880);
        assert_eq!(opus_packet_samples(&[0b1111_1011, 3]), 2880);
        assert_eq!(opus_packet_samples(&[0b1111_1001]), 1920);
        assert_eq!(opus_packet_samples(&[]), 0);
    }

    #[test]
    fn a_container_cannot_hide_the_real_length() {
        let bytes = include_bytes!("fixtures/voice-20s.mp4").to_vec();
        let ms = probe_voice_duration_ms(bytes).expect("duration");
        assert!((19_000..=20_500).contains(&ms), "{ms}");
    }

    #[test]
    fn a_spooled_upload_is_probed_from_disk_and_removed() {
        let spool = super::VoiceSpool::create().expect("spool");
        let bytes = include_bytes!("fixtures/voice-4s.webm");
        for chunk in bytes.chunks(1000) {
            spool.write(chunk).expect("write");
        }
        let path = spool.path.clone();
        let ms = spool.probe_duration_ms().expect("duration");
        assert!((3500..=4300).contains(&ms), "{ms}");
        drop(spool);
        assert!(!path.exists());
    }

    #[test]
    fn a_truncated_recording_is_rejected() {
        let bytes = include_bytes!("fixtures/voice-4s.webm");
        assert_eq!(probe_voice_duration_ms(bytes[..bytes.len() / 2].to_vec()), None);
    }

    #[test]
    fn rejects_bytes_that_are_not_audio() {
        assert_eq!(probe_voice_duration_ms(b"not audio at all".to_vec()), None);
    }
}
