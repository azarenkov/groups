use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

use symphonia::core::codecs::{CODEC_TYPE_NULL, CODEC_TYPE_OPUS};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::voice_probe::opus_packet_samples;

const OPUS_RATE: u64 = 48_000;
const MAX_BOXES: usize = 100_000;

pub fn probe_video_note_ms(media: Box<dyn MediaSource>, content_type: &str) -> Option<i64> {
    match content_type {
        "video/webm" => probe_webm(media),
        "video/mp4" => probe_mp4(media),
        _ => None,
    }
}

fn probe_webm(media: Box<dyn MediaSource>) -> Option<i64> {
    let source = MediaSourceStream::new(media, Default::default());
    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), source, &FormatOptions::default(), &MetadataOptions::default())
        .ok()?;
    let mut format = probed.format;
    let tracks = format.tracks().to_vec();
    let video = tracks.iter().filter(|t| t.codec_params.codec == CODEC_TYPE_NULL).count();
    let audio = tracks.iter().filter(|t| t.codec_params.codec == CODEC_TYPE_OPUS).count();
    if video != 1 || audio > 1 || video + audio != tracks.len() {
        return None;
    }
    let mut ends: HashMap<u32, u64> = HashMap::new();
    let mut opus_samples: u64 = 0;
    let mut packets = 0usize;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return None,
        };
        let track = tracks.iter().find(|t| t.id == packet.track_id())?;
        packets += 1;
        let end = ends.entry(track.id).or_insert(0);
        *end = (*end).max(packet.ts.saturating_add(packet.dur));
        if track.codec_params.codec == CODEC_TYPE_OPUS {
            opus_samples = opus_samples.saturating_add(opus_packet_samples(&packet.data));
        }
    }
    if packets == 0 {
        return None;
    }
    let mut longest = (opus_samples.saturating_mul(1000) / OPUS_RATE) as i64;
    for track in &tracks {
        let (Some(time_base), Some(&end)) = (track.codec_params.time_base, ends.get(&track.id)) else {
            continue;
        };
        let time = time_base.calc_time(end);
        longest = longest.max((time.seconds as f64 * 1000.0 + time.frac * 1000.0).round() as i64);
    }
    Some(longest)
}

#[derive(Default, Clone)]
struct Mp4Track {
    handler: [u8; 4],
    timescale: u32,
    media_duration: u64,
    default_duration: u32,
    end: u64,
}

struct Reader<R: Read + Seek> {
    inner: R,
    boxes: usize,
}

impl<R: Read + Seek> Reader<R> {
    fn bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut buf = [0u8; N];
        self.inner.read_exact(&mut buf).ok()?;
        Some(buf)
    }
    fn u32(&mut self) -> Option<u32> {
        self.bytes::<4>().map(u32::from_be_bytes)
    }
    fn u64(&mut self) -> Option<u64> {
        self.bytes::<8>().map(u64::from_be_bytes)
    }
    fn pos(&mut self) -> Option<u64> {
        self.inner.stream_position().ok()
    }
    fn seek(&mut self, to: u64) -> Option<()> {
        self.inner.seek(SeekFrom::Start(to)).ok().map(|_| ())
    }
    fn skip(&mut self, n: u64) -> Option<()> {
        self.inner.seek(SeekFrom::Current(n as i64)).ok().map(|_| ())
    }
    fn header(&mut self, limit: u64) -> Option<([u8; 4], u64, u64)> {
        let start = self.pos()?;
        if start + 8 > limit {
            return None;
        }
        self.boxes += 1;
        if self.boxes > MAX_BOXES {
            return None;
        }
        let size = self.u32()? as u64;
        let kind = self.bytes::<4>()?;
        let size = match size {
            0 => limit - start,
            1 => self.u64()?,
            n => n,
        };
        let end = start.checked_add(size)?;
        if size < 8 || end > limit {
            return None;
        }
        Some((kind, self.pos()?, end))
    }
    fn full_header(&mut self) -> Option<(u8, u32)> {
        let v = self.u32()?;
        Some(((v >> 24) as u8, v & 0x00FF_FFFF))
    }
}

pub fn probe_mp4(mut media: Box<dyn MediaSource>) -> Option<i64> {
    let len = media.byte_len().or_else(|| media.seek(SeekFrom::End(0)).ok())?;
    media.seek(SeekFrom::Start(0)).ok()?;
    let mut r = Reader { inner: media, boxes: 0 };
    let mut tracks: HashMap<u32, Mp4Track> = HashMap::new();
    let mut first = true;
    while r.pos()? < len {
        let (kind, _, end) = r.header(len)?;
        if first && &kind != b"ftyp" {
            return None;
        }
        first = false;
        match &kind {
            b"moov" => parse_moov(&mut r, end, &mut tracks)?,
            b"moof" => parse_moof(&mut r, end, &mut tracks)?,
            _ => {}
        }
        r.seek(end)?;
    }
    let video = tracks.values().filter(|t| &t.handler == b"vide").count();
    let audio = tracks.values().filter(|t| &t.handler == b"soun").count();
    if video != 1 || audio > 1 || video + audio != tracks.len() {
        return None;
    }
    let mut longest = 0i64;
    for track in tracks.values() {
        if track.timescale == 0 {
            return None;
        }
        let units = track.end.max(track.media_duration);
        longest = longest.max((units.saturating_mul(1000) / track.timescale as u64) as i64);
    }
    Some(longest)
}

fn parse_moov<R: Read + Seek>(r: &mut Reader<R>, limit: u64, tracks: &mut HashMap<u32, Mp4Track>) -> Option<()> {
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        match &kind {
            b"trak" => {
                let (id, track) = parse_trak(r, end)?;
                if tracks.insert(id, track).is_some() {
                    return None;
                }
            }
            b"mvex" => {
                while r.pos()? < end {
                    let (inner, _, inner_end) = r.header(end)?;
                    if &inner == b"trex" {
                        r.full_header()?;
                        let id = r.u32()?;
                        r.skip(4)?;
                        let duration = r.u32()?;
                        if let Some(track) = tracks.get_mut(&id) {
                            track.default_duration = duration;
                        }
                    }
                    r.seek(inner_end)?;
                }
            }
            _ => {}
        }
        r.seek(end)?;
    }
    Some(())
}

fn parse_trak<R: Read + Seek>(r: &mut Reader<R>, limit: u64) -> Option<(u32, Mp4Track)> {
    let mut id = None;
    let mut track = Mp4Track::default();
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        match &kind {
            b"tkhd" => {
                let (version, _) = r.full_header()?;
                r.skip(if version == 1 { 16 } else { 8 })?;
                id = Some(r.u32()?);
            }
            b"mdia" => {
                while r.pos()? < end {
                    let (inner, _, inner_end) = r.header(end)?;
                    match &inner {
                        b"mdhd" => {
                            let (version, _) = r.full_header()?;
                            if version == 1 {
                                r.skip(16)?;
                                track.timescale = r.u32()?;
                                track.media_duration = r.u64()?;
                            } else {
                                r.skip(8)?;
                                track.timescale = r.u32()?;
                                track.media_duration = r.u32()? as u64;
                            }
                        }
                        b"hdlr" => {
                            r.full_header()?;
                            r.skip(4)?;
                            track.handler = r.bytes::<4>()?;
                        }
                        _ => {}
                    }
                    r.seek(inner_end)?;
                }
            }
            _ => {}
        }
        r.seek(end)?;
    }
    Some((id?, track))
}

fn parse_moof<R: Read + Seek>(r: &mut Reader<R>, limit: u64, tracks: &mut HashMap<u32, Mp4Track>) -> Option<()> {
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        if &kind == b"traf" {
            parse_traf(r, end, tracks)?;
        }
        r.seek(end)?;
    }
    Some(())
}

fn parse_traf<R: Read + Seek>(r: &mut Reader<R>, limit: u64, tracks: &mut HashMap<u32, Mp4Track>) -> Option<()> {
    let mut id = None;
    let mut default_duration = None;
    let mut base: u64 = 0;
    let mut total: u64 = 0;
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        match &kind {
            b"tfhd" => {
                let (_, flags) = r.full_header()?;
                id = Some(r.u32()?);
                if flags & 0x01 != 0 {
                    r.skip(8)?;
                }
                if flags & 0x02 != 0 {
                    r.skip(4)?;
                }
                if flags & 0x08 != 0 {
                    default_duration = Some(r.u32()?);
                }
            }
            b"tfdt" => {
                let (version, _) = r.full_header()?;
                base = if version == 1 { r.u64()? } else { r.u32()? as u64 };
            }
            b"trun" => {
                let (_, flags) = r.full_header()?;
                let count = r.u32()? as u64;
                if flags & 0x01 != 0 {
                    r.skip(4)?;
                }
                if flags & 0x04 != 0 {
                    r.skip(4)?;
                }
                let track = tracks.get(&id?)?;
                let fallback = default_duration.unwrap_or(track.default_duration) as u64;
                let per_sample = [0x100u32, 0x200, 0x400, 0x800]
                    .iter()
                    .filter(|bit| flags & **bit != 0)
                    .count() as u64
                    * 4;
                if count.checked_mul(per_sample)? > end.saturating_sub(r.pos()?) {
                    return None;
                }
                for _ in 0..count {
                    let duration = if flags & 0x100 != 0 { r.u32()? as u64 } else { fallback };
                    total = total.saturating_add(duration);
                    r.skip(per_sample - if flags & 0x100 != 0 { 4 } else { 0 })?;
                }
            }
            _ => {}
        }
        r.seek(end)?;
    }
    let track = tracks.get_mut(&id?)?;
    track.end = track.end.max(base.saturating_add(total));
    Some(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::probe_video_note_ms;

    fn probe(bytes: &[u8], kind: &str) -> Option<i64> {
        probe_video_note_ms(Box::new(Cursor::new(bytes.to_vec())), kind)
    }

    #[test]
    fn reads_a_chrome_webm_video_note() {
        let ms = probe(include_bytes!("fixtures/video-note-5s.webm"), "video/webm").expect("duration");
        assert!((4500..=5500).contains(&ms), "{ms}");
    }

    #[test]
    fn reads_a_fragmented_mp4_video_note() {
        let ms = probe(include_bytes!("fixtures/video-note-5s.mp4"), "video/mp4").expect("duration");
        assert!((4500..=5500).contains(&ms), "{ms}");
    }

    #[test]
    fn rejects_audio_only_files_and_truncation() {
        assert_eq!(probe(include_bytes!("fixtures/voice-4s.webm"), "video/webm"), None);
        let mp4 = include_bytes!("fixtures/video-note-5s.mp4");
        assert_eq!(probe(&mp4[..40], "video/mp4"), None);
        assert_eq!(probe(b"not a video", "video/mp4"), None);
    }
}
