use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

use symphonia::core::codecs::{CODEC_TYPE_NULL, CODEC_TYPE_OPUS};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::codec_dims::{h264_sps_dims, vp8_frame_dims, vp9_frame_dims};
use super::voice_probe::{opus_packet_samples, opus_packet_valid};

const OPUS_RATE: u64 = 48_000;
const MAX_BOXES: usize = 100_000;
const MAX_SAMPLES: u64 = 200_000;
const WEBM_HEADER_BYTES: u64 = 256 * 1024;
const MAX_NAL_UNITS: u64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoProbe {
    pub duration_ms: i64,
    pub width: u32,
    pub height: u32,
}

pub fn probe_video_note(media: Box<dyn MediaSource>, content_type: &str) -> Option<VideoProbe> {
    match content_type {
        "video/webm" => probe_webm(media),
        "video/mp4" => probe_mp4(media),
        _ => None,
    }
}

pub fn probe_video_note_ms(media: Box<dyn MediaSource>, content_type: &str) -> Option<i64> {
    probe_video_note(media, content_type).map(|p| p.duration_ms)
}

fn read_vint(buf: &[u8], pos: &mut usize, keep_marker: bool) -> Option<u64> {
    let first = *buf.get(*pos)?;
    let len = first.leading_zeros() as usize + 1;
    if len > 8 || *pos + len > buf.len() {
        return None;
    }
    let mut value = if keep_marker { first as u64 } else { (first as u64) & ((1u64 << (8 - len)) - 1) };
    for i in 1..len {
        value = (value << 8) | buf[*pos + i] as u64;
    }
    *pos += len;
    Some(value)
}

fn read_element(buf: &[u8], pos: &mut usize) -> Option<(u64, usize, usize)> {
    let start = *pos;
    let id = read_vint(buf, pos, true)?;
    let first = *buf.get(*pos)?;
    let len = first.leading_zeros() as usize + 1;
    let size = read_vint(buf, pos, false)?;
    let unknown = size == (1u64 << (7 * len)) - 1;
    let end = if unknown { buf.len() } else { (*pos).checked_add(size as usize)? };
    if start >= *pos {
        return None;
    }
    Some((id, *pos, end.min(buf.len()).max(*pos)))
}

fn read_uint(buf: &[u8]) -> Option<u32> {
    if buf.is_empty() || buf.len() > 4 {
        return None;
    }
    Some(buf.iter().fold(0u32, |acc, b| (acc << 8) | *b as u32))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebmVideoCodec {
    Vp8,
    Vp9,
}

fn webm_video_codec(buf: &[u8]) -> Option<WebmVideoCodec> {
    let mut pos = 0;
    let (id, _, end) = read_element(buf, &mut pos)?;
    if id != 0x1A45_DFA3 {
        return None;
    }
    pos = end;
    let (id, body, _) = read_element(buf, &mut pos)?;
    if id != 0x1853_8067 {
        return None;
    }
    pos = body;
    while pos < buf.len() {
        let (id, body, end) = read_element(buf, &mut pos)?;
        match id {
            0x1654_AE6B => return tracks_video_codec(&buf[body..end]),
            0x1F43_B675 => return None,
            _ => pos = end,
        }
    }
    None
}

fn tracks_video_codec(buf: &[u8]) -> Option<WebmVideoCodec> {
    let mut pos = 0;
    while pos < buf.len() {
        let (id, body, end) = read_element(buf, &mut pos)?;
        if id == 0xAE {
            let mut inner = body;
            let mut codec = None;
            let mut video = false;
            while inner < end {
                let (child, child_body, child_end) = read_element(&buf[..end], &mut inner)?;
                match child {
                    0x86 => codec = Some(&buf[child_body..child_end]),
                    0xE0 => video = true,
                    _ => {}
                }
                inner = child_end;
            }
            if video {
                return match codec? {
                    b"V_VP8" => Some(WebmVideoCodec::Vp8),
                    b"V_VP9" => Some(WebmVideoCodec::Vp9),
                    _ => None,
                };
            }
        }
        pos = end;
    }
    None
}

pub fn webm_dimensions(buf: &[u8]) -> Option<(u32, u32)> {
    let mut pos = 0;
    let (id, _, end) = read_element(buf, &mut pos)?;
    if id != 0x1A45_DFA3 {
        return None;
    }
    pos = end;
    let (id, body, _) = read_element(buf, &mut pos)?;
    if id != 0x1853_8067 {
        return None;
    }
    pos = body;
    while pos < buf.len() {
        let (id, body, end) = read_element(buf, &mut pos)?;
        match id {
            0x1654_AE6B => return tracks_dimensions(&buf[body..end]),
            0x1F43_B675 => return None,
            _ => pos = end,
        }
    }
    None
}

fn tracks_dimensions(buf: &[u8]) -> Option<(u32, u32)> {
    let mut pos = 0;
    while pos < buf.len() {
        let (id, body, end) = read_element(buf, &mut pos)?;
        if id == 0xAE {
            let mut inner = body;
            while inner < end {
                let (child, child_body, child_end) = read_element(&buf[..end], &mut inner)?;
                if child == 0xE0 {
                    let mut width = None;
                    let mut height = None;
                    let mut v = child_body;
                    while v < child_end {
                        let (field, field_body, field_end) = read_element(&buf[..child_end], &mut v)?;
                        match field {
                            0xB0 => width = read_uint(&buf[field_body..field_end]),
                            0xBA => height = read_uint(&buf[field_body..field_end]),
                            _ => {}
                        }
                        v = field_end;
                    }
                    return Some((width?, height?));
                }
                inner = child_end;
            }
        }
        pos = end;
    }
    None
}

fn probe_webm(mut media: Box<dyn MediaSource>) -> Option<VideoProbe> {
    let mut header = Vec::new();
    (&mut media).take(WEBM_HEADER_BYTES).read_to_end(&mut header).ok()?;
    let (width, height) = webm_dimensions(&header)?;
    let codec = webm_video_codec(&header)?;
    media.seek(SeekFrom::Start(0)).ok()?;
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
    let mut keyframes = 0usize;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return None,
        };
        let track = tracks.iter().find(|t| t.id == packet.track_id())?;
        packets += 1;
        if packets as u64 > MAX_SAMPLES {
            return None;
        }
        if track.codec_params.codec == CODEC_TYPE_NULL {
            let dims = match codec {
                WebmVideoCodec::Vp8 => vp8_frame_dims(&packet.data)?,
                WebmVideoCodec::Vp9 => vp9_frame_dims(&packet.data)?,
            };
            if let Some(dims) = dims {
                if dims != (width, height) {
                    return None;
                }
                keyframes += 1;
            }
        }
        let end = ends.entry(track.id).or_insert(0);
        *end = (*end).max(packet.ts.saturating_add(packet.dur));
        if track.codec_params.codec == CODEC_TYPE_OPUS {
            if !opus_packet_valid(&packet.data) {
                return None;
            }
            opus_samples = opus_samples.saturating_add(opus_packet_samples(&packet.data));
        }
    }
    if packets == 0 || keyframes == 0 {
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
    Some(VideoProbe { duration_ms: longest, width, height })
}

#[derive(Default, Clone)]
struct Mp4Track {
    handler: [u8; 4],
    timescale: u32,
    media_duration: u64,
    default_duration: u32,
    default_size: u32,
    end: u64,
    width: u32,
    height: u32,
    coded: Option<(u32, u32)>,
    nal_length_size: u8,
}

struct Reader<R: Read + Seek> {
    inner: R,
    boxes: usize,
    samples: u64,
    sample_bytes: u64,
    mdat_ranges: Vec<(u64, u64)>,
    sample_ranges: Vec<(u64, u64)>,
    video_samples: Vec<(u64, u64)>,
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

pub fn probe_mp4(mut media: Box<dyn MediaSource>) -> Option<VideoProbe> {
    let len = media.byte_len().or_else(|| media.seek(SeekFrom::End(0)).ok())?;
    media.seek(SeekFrom::Start(0)).ok()?;
    let mut r = Reader {
        inner: media,
        boxes: 0,
        samples: 0,
        sample_bytes: 0,
        mdat_ranges: Vec::new(),
        sample_ranges: Vec::new(),
        video_samples: Vec::new(),
    };
    let mut tracks: HashMap<u32, Mp4Track> = HashMap::new();
    let mut first = true;
    while r.pos()? < len {
        let box_start = r.pos()?;
        let (kind, body, end) = r.header(len)?;
        if first && &kind != b"ftyp" {
            return None;
        }
        first = false;
        match &kind {
            b"moov" => parse_moov(&mut r, end, &mut tracks)?,
            b"moof" => parse_moof(&mut r, end, box_start, &mut tracks)?,
            b"mdat" => r.mdat_ranges.push((body, end)),
            _ => {}
        }
        r.seek(end)?;
    }
    if r.sample_bytes == 0 || r.sample_ranges.is_empty() {
        return None;
    }
    let mut media = std::mem::take(&mut r.mdat_ranges);
    media.sort_unstable();
    let inside = |(start, end): &(u64, u64)| {
        if start > end {
            return false;
        }
        let at = media.partition_point(|(from, _)| from <= start);
        at > 0 && *end <= media[at - 1].1
    };
    if !r.sample_ranges.iter().all(inside) {
        return None;
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
    let video_track = tracks.values().find(|t| &t.handler == b"vide")?;
    let (width, height) = video_track.coded?;
    let samples = std::mem::take(&mut r.video_samples);
    if !avc_samples_valid(&mut r, &samples, video_track.nal_length_size) {
        return None;
    }
    if (width, height) != (video_track.width, video_track.height) {
        return None;
    }
    Some(VideoProbe { duration_ms: longest, width, height })
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
                        let size = r.u32()?;
                        if let Some(track) = tracks.get_mut(&id) {
                            track.default_duration = duration;
                            track.default_size = size;
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
                r.skip(if version == 1 { 64 } else { 60 })?;
                track.width = r.u32()? >> 16;
                track.height = r.u32()? >> 16;
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
                                let duration = r.u64()?;
                                track.media_duration = if duration == u64::MAX { 0 } else { duration };
                            } else {
                                r.skip(8)?;
                                track.timescale = r.u32()?;
                                let duration = r.u32()?;
                                track.media_duration = if duration == u32::MAX { 0 } else { duration as u64 };
                            }
                        }
                        b"hdlr" => {
                            r.full_header()?;
                            r.skip(4)?;
                            track.handler = r.bytes::<4>()?;
                        }
                        b"minf" => {
                            if let Some((dims, nal_length_size)) = find_avc_dims(r, inner_end)? {
                                track.coded = Some(dims);
                                track.nal_length_size = nal_length_size;
                            }
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

type AvcConfig = ((u32, u32), u8);

fn find_avc_dims<R: Read + Seek>(r: &mut Reader<R>, limit: u64) -> Option<Option<AvcConfig>> {
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        match &kind {
            b"stbl" => {
                if let Some(dims) = find_avc_dims(r, end)? {
                    return Some(Some(dims));
                }
            }
            b"stsd" => {
                r.full_header()?;
                let count = r.u32()?;
                if count != 1 {
                    return None;
                }
                let (entry, _, entry_end) = r.header(end)?;
                if &entry == b"mp4a" || &entry == b"Opus" {
                    return Some(None);
                }
                if &entry != b"avc1" {
                    return None;
                }
                r.skip(78)?;
                while r.pos()? < entry_end {
                    let (child, _, child_end) = r.header(entry_end)?;
                    if &child == b"avcC" {
                        r.skip(4)?;
                        let nal_length_size = (r.bytes::<1>()?[0] & 0x03) + 1;
                        let sps_count = r.bytes::<1>()?[0] & 0x1F;
                        if sps_count == 0 {
                            return None;
                        }
                        let mut dims = None;
                        for _ in 0..sps_count {
                            let len = u16::from_be_bytes(r.bytes::<2>()?) as u64;
                            if len == 0 || r.pos()? + len > child_end {
                                return None;
                            }
                            let mut sps = vec![0u8; len as usize];
                            r.inner.read_exact(&mut sps).ok()?;
                            let next = h264_sps_dims(&sps)?;
                            if dims.is_some_and(|d| d != next) {
                                return None;
                            }
                            dims = Some(next);
                        }
                        return Some(dims.map(|d| (d, nal_length_size)));
                    }
                    r.seek(child_end)?;
                }
                return None;
            }
            _ => {}
        }
        r.seek(end)?;
    }
    Some(None)
}

fn avc_samples_valid<R: Read + Seek>(r: &mut Reader<R>, samples: &[(u64, u64)], nal_length_size: u8) -> bool {
    let check = |r: &mut Reader<R>| -> Option<bool> {
        if !(1..=4).contains(&nal_length_size) || samples.is_empty() {
            return None;
        }
        let mut first = true;
        let mut units: u64 = 0;
        for &(offset, size) in samples {
            let end = offset.checked_add(size)?;
            let mut pos = offset;
            let mut idr = false;
            if size == 0 {
                return None;
            }
            while pos < end {
                units += 1;
                if units > MAX_NAL_UNITS {
                    return None;
                }
                r.seek(pos)?;
                let mut len_bytes = [0u8; 4];
                r.inner.read_exact(&mut len_bytes[4 - nal_length_size as usize..]).ok()?;
                let len = u32::from_be_bytes(len_bytes) as u64;
                let header = r.bytes::<1>()?[0];
                let body_end = pos.checked_add(nal_length_size as u64)?.checked_add(len)?;
                if len == 0 || body_end > end || header & 0x80 != 0 {
                    return None;
                }
                let kind = header & 0x1F;
                if !(1..=23).contains(&kind) {
                    return None;
                }
                idr |= kind == 5;
                pos = body_end;
            }
            if first && !idr {
                return None;
            }
            first = false;
        }
        Some(true)
    };
    check(r).unwrap_or(false)
}

fn parse_moof<R: Read + Seek>(
    r: &mut Reader<R>,
    limit: u64,
    moof_start: u64,
    tracks: &mut HashMap<u32, Mp4Track>,
) -> Option<()> {
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        if &kind == b"traf" {
            parse_traf(r, end, moof_start, tracks)?;
        }
        r.seek(end)?;
    }
    Some(())
}

fn parse_traf<R: Read + Seek>(
    r: &mut Reader<R>,
    limit: u64,
    moof_start: u64,
    tracks: &mut HashMap<u32, Mp4Track>,
) -> Option<()> {
    let mut id = None;
    let mut base_offset = moof_start;
    let mut cursor: Option<u64> = None;
    let mut default_duration = None;
    let mut default_size = None;
    let mut base: u64 = 0;
    let mut total: u64 = 0;
    while r.pos()? < limit {
        let (kind, _, end) = r.header(limit)?;
        match &kind {
            b"tfhd" => {
                let (_, flags) = r.full_header()?;
                id = Some(r.u32()?);
                if flags & 0x01 != 0 {
                    base_offset = r.u64()?;
                }
                if flags & 0x02 != 0 {
                    r.skip(4)?;
                }
                if flags & 0x08 != 0 {
                    default_duration = Some(r.u32()?);
                }
                if flags & 0x10 != 0 {
                    default_size = Some(r.u32()?);
                }
            }
            b"tfdt" => {
                let (version, _) = r.full_header()?;
                base = if version == 1 { r.u64()? } else { r.u32()? as u64 };
            }
            b"trun" => {
                let (_, flags) = r.full_header()?;
                let count = r.u32()? as u64;
                let run_start = if flags & 0x01 != 0 {
                    let offset = r.u32()? as i32 as i64;
                    u64::try_from(base_offset as i64 + offset).ok()?
                } else {
                    cursor.unwrap_or(base_offset)
                };
                let bytes_before = r.sample_bytes;
                if flags & 0x04 != 0 {
                    r.skip(4)?;
                }
                let track = tracks.get(&id?)?;
                let fallback = default_duration.unwrap_or(track.default_duration) as u64;
                let fallback_size = default_size.unwrap_or(track.default_size) as u64;
                let is_video = &track.handler == b"vide";
                let per_sample = [0x100u32, 0x200, 0x400, 0x800]
                    .iter()
                    .filter(|bit| flags & **bit != 0)
                    .count() as u64
                    * 4;
                r.samples = r.samples.saturating_add(count);
                if r.samples > MAX_SAMPLES {
                    return None;
                }
                if count.checked_mul(per_sample)? > end.saturating_sub(r.pos()?) {
                    return None;
                }
                if per_sample == 0 {
                    total = total.saturating_add(count.saturating_mul(fallback));
                    r.sample_bytes = r.sample_bytes.saturating_add(count.saturating_mul(fallback_size));
                    if is_video {
                        for i in 0..count {
                            r.video_samples.push((run_start.checked_add(i.checked_mul(fallback_size)?)?, fallback_size));
                        }
                    }
                } else {
                    let mut sample_at = run_start;
                    for _ in 0..count {
                        let duration = if flags & 0x100 != 0 { r.u32()? as u64 } else { fallback };
                        let size = if flags & 0x200 != 0 { r.u32()? as u64 } else { fallback_size };
                        if flags & 0x400 != 0 {
                            r.skip(4)?;
                        }
                        if flags & 0x800 != 0 {
                            r.skip(4)?;
                        }
                        total = total.saturating_add(duration);
                        r.sample_bytes = r.sample_bytes.saturating_add(size);
                        if is_video {
                            r.video_samples.push((sample_at, size));
                        }
                        sample_at = sample_at.checked_add(size)?;
                    }
                }
                let run_bytes = r.sample_bytes - bytes_before;
                let run_end = run_start.checked_add(run_bytes)?;
                r.sample_ranges.push((run_start, run_end));
                cursor = Some(run_end);
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
    fn reads_the_frame_size_of_both_containers() {
        use super::probe_video_note;
        let webm = probe_video_note(
            Box::new(Cursor::new(include_bytes!("fixtures/video-note-5s.webm").to_vec())),
            "video/webm",
        )
        .expect("webm");
        assert_eq!((webm.width, webm.height), (384, 384));
        let mp4 = probe_video_note(
            Box::new(Cursor::new(include_bytes!("fixtures/video-note-5s.mp4").to_vec())),
            "video/mp4",
        )
        .expect("mp4");
        assert_eq!((mp4.width, mp4.height), (384, 384));
    }

    #[test]
    fn a_huge_sample_count_is_rejected_without_iterating() {
        let mut mp4 = include_bytes!("fixtures/video-note-5s.mp4").to_vec();
        let at = mp4.windows(4).position(|w| w == b"trun").expect("trun");
        mp4[at + 4..at + 8].copy_from_slice(&[0, 0, 0, 0]);
        mp4[at + 8..at + 12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(probe(&mp4, "video/mp4"), None);
    }

    #[test]
    fn reads_the_coded_size_of_a_vp9_note() {
        use super::probe_video_note;
        let vp9 = probe_video_note(
            Box::new(Cursor::new(include_bytes!("fixtures/video-note-5s-vp9.webm").to_vec())),
            "video/webm",
        )
        .expect("vp9");
        assert_eq!((vp9.width, vp9.height), (384, 384));
    }

    #[test]
    fn a_forged_container_size_does_not_hide_the_coded_size() {
        let mut webm = include_bytes!("fixtures/video-note-5s.webm").to_vec();
        let at = webm.windows(3).position(|w| w == [0xB0, 0x82, 0x01]).expect("PixelWidth");
        webm[at + 2..at + 4].copy_from_slice(&640u16.to_be_bytes());
        assert_eq!(probe(&webm, "video/webm"), None);
    }

    #[test]
    fn an_mp4_without_media_data_is_rejected() {
        let mp4 = include_bytes!("fixtures/video-note-5s.mp4");
        let mut stripped = Vec::new();
        let mut pos = 0;
        while pos + 8 <= mp4.len() {
            let size = u32::from_be_bytes(mp4[pos..pos + 4].try_into().unwrap()) as usize;
            if &mp4[pos + 4..pos + 8] != b"mdat" {
                stripped.extend_from_slice(&mp4[pos..pos + size]);
            }
            pos += size;
        }
        assert_eq!(probe(&stripped, "video/mp4"), None);
    }

    #[test]
    fn samples_pointing_outside_the_media_data_are_rejected() {
        let mut mp4 = include_bytes!("fixtures/video-note-5s.mp4").to_vec();
        let at = mp4.windows(4).position(|w| w == b"trun").expect("trun");
        let flags = u32::from_be_bytes(mp4[at + 4..at + 8].try_into().unwrap()) & 0x00FF_FFFF;
        assert!(flags & 0x01 != 0);
        mp4[at + 12..at + 16].copy_from_slice(&0x7000_0000u32.to_be_bytes());
        assert_eq!(probe(&mp4, "video/mp4"), None);
    }

    #[test]
    fn zeroed_media_data_is_rejected() {
        let mut mp4 = include_bytes!("fixtures/video-note-5s.mp4").to_vec();
        let mut pos = 0;
        while pos + 8 <= mp4.len() {
            let size = u32::from_be_bytes(mp4[pos..pos + 4].try_into().unwrap()) as usize;
            if &mp4[pos + 4..pos + 8] == b"mdat" {
                for b in &mut mp4[pos + 8..pos + size] {
                    *b = 0;
                }
            }
            pos += size;
        }
        assert_eq!(probe(&mp4, "video/mp4"), None);
    }

    #[test]
    fn rejects_audio_only_files_and_truncation() {
        assert_eq!(probe(include_bytes!("fixtures/voice-4s.webm"), "video/webm"), None);
        let mp4 = include_bytes!("fixtures/video-note-5s.mp4");
        assert_eq!(probe(&mp4[..40], "video/mp4"), None);
        assert_eq!(probe(b"not a video", "video/mp4"), None);
    }
}
