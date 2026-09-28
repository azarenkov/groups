struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - (self.pos % 8))) & 1;
        self.pos += 1;
        Some(bit as u32)
    }

    fn bits(&mut self, n: u32) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..n {
            value = (value << 1) | self.bit()?;
        }
        Some(value)
    }

    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0u32;
        while self.bit()? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let rest = self.bits(zeros)?;
        Some(((1u64 << zeros) - 1 + rest as u64).min(u32::MAX as u64) as u32)
    }

    fn se(&mut self) -> Option<i64> {
        let k = self.ue()? as i64;
        Some(if k % 2 == 1 { (k + 1) / 2 } else { -(k / 2) })
    }
}

pub fn vp8_frame_dims(frame: &[u8]) -> Option<Option<(u32, u32)>> {
    if frame.len() < 3 {
        return None;
    }
    let tag = frame[0] as u32 | (frame[1] as u32) << 8 | (frame[2] as u32) << 16;
    let version = (tag >> 1) & 0x07;
    let first_partition = (tag >> 5) as usize;
    let header = if tag & 0x01 == 0 { 10 } else { 3 };
    if version > 3 || first_partition == 0 || header + first_partition > frame.len() {
        return None;
    }
    if tag & 0x01 != 0 {
        return Some(None);
    }
    if frame.len() < 10 || frame[3..6] != [0x9D, 0x01, 0x2A] {
        return None;
    }
    let width = u16::from_le_bytes([frame[6], frame[7]]) as u32 & 0x3FFF;
    let height = u16::from_le_bytes([frame[8], frame[9]]) as u32 & 0x3FFF;
    Some(Some((width, height)))
}

fn vp9_superframe_ok(frame: &[u8]) -> bool {
    let Some(&last) = frame.last() else {
        return false;
    };
    if last & 0xE0 != 0xC0 {
        return true;
    }
    let frames = (last & 0x07) as usize + 1;
    let size_bytes = ((last >> 3) & 0x03) as usize + 1;
    let index_len = 2 + frames * size_bytes;
    if frame.len() < index_len || frame[frame.len() - index_len] != last {
        return false;
    }
    let mut total = 0usize;
    let sizes = &frame[frame.len() - index_len + 1..frame.len() - 1];
    for chunk in sizes.chunks(size_bytes) {
        let size = chunk.iter().rev().fold(0usize, |acc, b| (acc << 8) | *b as usize);
        if size == 0 {
            return false;
        }
        total += size;
    }
    total == frame.len() - index_len
}

fn vp9_color_config(b: &mut Bits, profile: u32) -> Option<()> {
    if profile >= 2 {
        b.bit()?;
    }
    let color_space = b.bits(3)?;
    if color_space != 7 {
        b.bit()?;
        if profile == 1 || profile == 3 {
            b.bits(3)?;
        }
    } else if profile == 1 || profile == 3 {
        b.bit()?;
    }
    Some(())
}

fn vp9_single_frame_dims(frame: &[u8]) -> Option<Option<(u32, u32)>> {
    let mut b = Bits::new(frame);
    if b.bits(2)? != 2 {
        return None;
    }
    let low = b.bit()?;
    let high = b.bit()?;
    let profile = (high << 1) | low;
    if profile == 3 && b.bit()? != 0 {
        return None;
    }
    if b.bit()? == 1 {
        return Some(None);
    }
    let frame_type = b.bit()?;
    let show_frame = b.bit()?;
    let error_resilient = b.bit()?;
    if frame_type == 0 {
        if b.bits(24)? != 0x49_83_42 {
            return None;
        }
        vp9_color_config(&mut b, profile)?;
    } else {
        let intra_only = if show_frame == 1 { 0 } else { b.bit()? };
        if error_resilient == 0 {
            b.bits(2)?;
        }
        if intra_only == 1 {
            if b.bits(24)? != 0x49_83_42 {
                return None;
            }
            if profile > 0 {
                vp9_color_config(&mut b, profile)?;
            }
            b.bits(8)?;
        } else {
            b.bits(8)?;
            for _ in 0..3 {
                b.bits(4)?;
            }
            for _ in 0..3 {
                if b.bit()? == 1 {
                    return Some(None);
                }
            }
        }
    }
    let width = b.bits(16)? + 1;
    let height = b.bits(16)? + 1;
    Some(Some((width, height)))
}

pub fn vp9_frame_dims(packet: &[u8]) -> Option<Option<(u32, u32)>> {
    if packet.len() < 2 || !vp9_superframe_ok(packet) {
        return None;
    }
    let last = *packet.last()?;
    let frames: Vec<&[u8]> = if last & 0xE0 == 0xC0 {
        let count = (last & 0x07) as usize + 1;
        let size_bytes = ((last >> 3) & 0x03) as usize + 1;
        let index_len = 2 + count * size_bytes;
        let sizes = &packet[packet.len() - index_len + 1..packet.len() - 1];
        let mut out = Vec::with_capacity(count);
        let mut at = 0usize;
        for chunk in sizes.chunks(size_bytes) {
            let size = chunk.iter().rev().fold(0usize, |acc, b| (acc << 8) | *b as usize);
            out.push(&packet[at..at + size]);
            at += size;
        }
        out
    } else {
        vec![packet]
    };
    let mut dims = None;
    for frame in frames {
        if let Some(next) = vp9_single_frame_dims(frame)? {
            if dims.is_some_and(|d| d != next) {
                return None;
            }
            dims = Some(next);
        }
    }
    Some(dims)
}

fn unescape_nal(nal: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(nal.len());
    let mut zeros = 0;
    for &byte in nal {
        if zeros >= 2 && byte == 0x03 {
            zeros = 0;
            continue;
        }
        zeros = if byte == 0 { zeros + 1 } else { 0 };
        out.push(byte);
    }
    out
}

fn skip_scaling_list(b: &mut Bits, size: usize) -> Option<()> {
    let mut last = 8i64;
    let mut next = 8i64;
    for _ in 0..size {
        if next != 0 {
            let delta = b.se()?;
            next = (last + delta + 256) % 256;
        }
        last = if next == 0 { last } else { next };
    }
    Some(())
}

pub fn h264_sps_dims(sps: &[u8]) -> Option<(u32, u32)> {
    let raw = unescape_nal(sps);
    let mut b = Bits::new(&raw);
    if b.bits(8)? & 0x1F != 7 {
        return None;
    }
    let profile = b.bits(8)?;
    b.bits(16)?;
    b.ue()?;
    let mut chroma_format = 1;
    let mut separate_planes = 0;
    if matches!(profile, 100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135) {
        chroma_format = b.ue()?;
        if chroma_format > 3 {
            return None;
        }
        if chroma_format == 3 {
            separate_planes = b.bit()?;
        }
        b.ue()?;
        b.ue()?;
        b.bit()?;
        if b.bit()? == 1 {
            let lists = if chroma_format == 3 { 12 } else { 8 };
            for i in 0..lists {
                if b.bit()? == 1 {
                    skip_scaling_list(&mut b, if i < 6 { 16 } else { 64 })?;
                }
            }
        }
    }
    b.ue()?;
    match b.ue()? {
        0 => {
            b.ue()?;
        }
        1 => {
            b.bit()?;
            b.se()?;
            b.se()?;
            let cycle = b.ue()?;
            if cycle > 255 {
                return None;
            }
            for _ in 0..cycle {
                b.se()?;
            }
        }
        2 => {}
        _ => return None,
    }
    b.ue()?;
    b.bit()?;
    let width_mbs = b.ue()? as u64 + 1;
    let height_units = b.ue()? as u64 + 1;
    let frame_mbs_only = b.bit()? as u64;
    if frame_mbs_only == 0 {
        b.bit()?;
    }
    b.bit()?;
    let (mut crop_x, mut crop_y) = (0u64, 0u64);
    if b.bit()? == 1 {
        let left = b.ue()? as u64;
        let right = b.ue()? as u64;
        let top = b.ue()? as u64;
        let bottom = b.ue()? as u64;
        let (unit_x, unit_y) = match (chroma_format, separate_planes) {
            (0, _) | (_, 1) => (1, 2 - frame_mbs_only),
            (1, _) => (2, 2 * (2 - frame_mbs_only)),
            (2, _) => (2, 2 - frame_mbs_only),
            _ => (1, 2 - frame_mbs_only),
        };
        crop_x = (left + right) * unit_x;
        crop_y = (top + bottom) * unit_y;
    }
    let width = (width_mbs * 16).checked_sub(crop_x)?;
    let height = ((2 - frame_mbs_only) * height_units * 16).checked_sub(crop_y)?;
    Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
}

pub fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if !data.starts_with(&[0xFF, 0xD8]) || !data.ends_with(&[0xFF, 0xD9]) {
        return None;
    }
    let mut pos = 2;
    let mut dims = None;
    while pos + 4 <= data.len() {
        if data[pos] != 0xFF {
            return None;
        }
        let marker = data[pos + 1];
        if marker == 0xFF {
            pos += 1;
            continue;
        }
        let len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        if len < 2 || pos + 2 + len > data.len() {
            return None;
        }
        let segment = &data[pos + 4..pos + 2 + len];
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            if segment.len() < 5 {
                return None;
            }
            let height = u16::from_be_bytes([segment[1], segment[2]]) as u32;
            let width = u16::from_be_bytes([segment[3], segment[4]]) as u32;
            dims = Some((width, height));
        }
        if marker == 0xDA {
            return dims.filter(|(w, h)| *w > 0 && *h > 0);
        }
        pos += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_vp8_keyframe_and_skips_inter_frames() {
        let mut key = vec![0x10, 0x02, 0x00, 0x9D, 0x01, 0x2A, 0x80, 0x01, 0x80, 0x01];
        key.extend_from_slice(&[0; 16]);
        assert_eq!(vp8_frame_dims(&key), Some(Some((384, 384))));
        assert_eq!(vp8_frame_dims(&[0x31, 0x00, 0x00, 1]), Some(None));
        assert_eq!(vp8_frame_dims(&[0x11, 0, 0]), None);
        assert_eq!(vp8_frame_dims(&[0xF1, 0xFF, 0xFF, 1]), None);
        assert_eq!(vp8_frame_dims(&[0x10, 0, 0, 1, 2, 3, 0, 0, 0, 0]), None);
    }

    #[test]
    fn a_broken_vp9_superframe_index_is_rejected() {
        assert_eq!(vp9_frame_dims(&[0x80, 0x00, 0xC0]), None);
        assert_eq!(vp9_frame_dims(&[0x86, 0x00, 0x00, 0x02]), Some(None));
    }

    #[test]
    fn a_jpeg_needs_a_frame_header_and_an_end() {
        assert_eq!(jpeg_dimensions(&[0xFF, 0xD8, 0xFF]), None);
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0xA0, 0x00, 0xA0, 0x03, 0x01, 0x22,
            0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00,
            0x00, 0x3F, 0x00, 0x12, 0x34, 0xFF, 0xD9,
        ];
        assert_eq!(jpeg_dimensions(&jpeg), Some((160, 160)));
        assert_eq!(jpeg_dimensions(&jpeg[..jpeg.len() - 2]), None);
    }

    #[test]
    fn a_vp9_intra_only_frame_reports_its_size() {
        let mut bits: Vec<u8> = Vec::new();
        let mut push = |value: u32, n: u32| {
            for i in (0..n).rev() {
                bits.push(((value >> i) & 1) as u8);
            }
        };
        push(2, 2);
        push(0, 2);
        push(0, 1);
        push(1, 1);
        push(0, 1);
        push(0, 1);
        push(1, 1);
        push(0, 2);
        push(0x49_83_42, 24);
        push(0xFF, 8);
        push(639, 16);
        push(479, 16);
        while !bits.len().is_multiple_of(8) {
            bits.push(0);
        }
        let bytes: Vec<u8> = bits.chunks(8).map(|c| c.iter().fold(0u8, |a, b| (a << 1) | b)).collect();
        assert_eq!(vp9_frame_dims(&bytes), Some(Some((640, 480))));
    }
}
