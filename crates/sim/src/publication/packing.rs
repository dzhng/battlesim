//! The compact arm's carrier: words already chosen for delivery written as
//! variable-width bit fields into raw u32 carriers, and the ground's tile
//! runs written through the same carrier.
use super::GROUND_FIELDS;

/// Ground keeps its canonical four-word logical rows. Only their final wire
/// tail changes: ordered tile deltas and exact byte marks share the carrier
/// writer with non-map groups, without a baseline or another staging buffer.
pub(super) fn pack_ground(values: &[f32], sink: &mut impl PackedSink) {
    let mut prior = 0;
    for row in values.as_chunks::<{ GROUND_FIELDS.len() }>().0 {
        let tile = row[0] as u32;
        let span = row[1] as u32;
        let a = row[2] as u32;
        let b = row[3] as u32;
        sink.integer(tile - prior);
        sink.put(span % 256, 8);
        sink.put(span / 256 - 1, 8);
        let marks = [a & 255, a >> 8, b & 255, (b >> 8) & 255, b >> 16];
        let mask = marks
            .iter()
            .enumerate()
            .fold(0, |mask, (i, &value)| mask | (u32::from(value != 0) << i));
        sink.put(mask, 5);
        for value in marks {
            if value != 0 {
                sink.put(value, 8);
            }
        }
        prior = tile;
    }
}

/// The compact carrier changes only serialization of an already selected word
/// form. No second span selection, byte buffer, or decoded state is retained.
pub(super) trait PackedSink {
    fn put(&mut self, value: u32, bits: usize);
    fn integer(&mut self, mut value: u32) {
        loop {
            let byte = value & 127;
            value >>= 7;
            self.put(byte | if value == 0 { 0 } else { 128 }, 8);
            if value == 0 {
                break;
            }
        }
    }
    fn literal(&mut self, value: f32, old: f32, predict: bool) {
        let raw = value.to_bits();
        let xor = raw ^ old.to_bits();
        let raw_bytes = (32 - raw.leading_zeros() as usize).div_ceil(8);
        let xor_bytes = (32 - xor.leading_zeros() as usize).div_ceil(8);
        let residual = predict && xor_bytes < raw_bytes;
        let bytes = if residual { xor_bytes } else { raw_bytes };
        self.put((bytes + if residual { 5 } else { 0 }) as u32, 4);
        if bytes != 0 {
            self.put(if residual { xor } else { raw }, bytes * 8);
        }
    }
}
pub(super) struct PackedCount(pub(super) usize);
impl PackedSink for PackedCount {
    fn put(&mut self, _: u32, bits: usize) {
        self.0 += bits;
    }
}
pub(super) struct PackedWriter<'a> {
    pub(super) out: &'a mut Vec<f32>,
    pub(super) pending: u64,
    pub(super) bits: usize,
}
impl PackedSink for PackedWriter<'_> {
    fn put(&mut self, value: u32, bits: usize) {
        self.pending |= (value as u64) << self.bits;
        self.bits += bits;
        if self.bits >= 32 {
            self.out.push(f32::from_bits(self.pending as u32));
            self.pending >>= 32;
            self.bits -= 32;
        }
    }
}
impl PackedWriter<'_> {
    pub(super) fn finish(self) {
        if self.bits != 0 {
            self.out.push(f32::from_bits(self.pending as u32));
        }
    }
}
