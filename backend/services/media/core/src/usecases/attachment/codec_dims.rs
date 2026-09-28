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
    let tag = *frame.first()?;
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

pub fn vp9_frame_dims(frame: &[u8]) -> Option<Option<(u32, u32)>> {
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
    b.bit()?;
    b.bit()?;
    if frame_type != 0 {
        return Some(None);
    }
    if b.bits(24)? != 0x49_83_42 {
        return None;
    }
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
    let width = b.bits(16)? + 1;
    let height = b.bits(16)? + 1;
    Some(Some((width, height)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_vp8_keyframe_and_skips_inter_frames() {
        let key = [0x10, 0x02, 0x00, 0x9D, 0x01, 0x2A, 0x80, 0x01, 0x80, 0x01];
        assert_eq!(vp8_frame_dims(&key), Some(Some((384, 384))));
        assert_eq!(vp8_frame_dims(&[0x11, 0, 0]), Some(None));
        assert_eq!(vp8_frame_dims(&[0x10, 0, 0, 1, 2, 3, 0, 0, 0, 0]), None);
    }
}
