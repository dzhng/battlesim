//! A group's delivery operations: changed word ranges over its old words,
//! or copies of exact old spans with new literals between them, measured
//! and written through one carrier sink so either encoding is sized by the
//! same pass that writes it.
use super::packing::{PackedCount, PackedSink};

pub(super) fn measure_group(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
) -> (usize, usize, u32) {
    let mut count = PackedCount(8);
    let (words, operations) =
        write_group_operations(encoding, values, old, stride, index, &mut count);
    if encoding == 0 {
        count.integer(operations);
    }
    (words, count.0.div_ceil(32), operations)
}
pub(super) fn pack_group(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
    operations: u32,
    sink: &mut impl PackedSink,
) {
    sink.put(encoding as u32, 8);
    if encoding == 0 {
        sink.integer(operations);
    }
    write_group_operations(encoding, values, old, stride, index, sink);
}
/// Measure the ordinary and compact representations together. In particular,
/// selecting a compact copy never adds another sparse-index search pass.
fn write_group_operations(
    encoding: u8,
    values: &[f32],
    old: &[f32],
    stride: usize,
    index: &[u32],
    sink: &mut impl PackedSink,
) -> (usize, u32) {
    let mut words = 0;
    let mut operations = 0;
    if encoding == 1 {
        for &value in values {
            sink.literal(value, 0.0, false);
        }
        words = values.len();
    } else if encoding == 0 {
        for (from, to) in changed_ranges(values, old) {
            operations += 1;
            words += 2 + to - from;
            sink.integer(from as u32);
            sink.integer((to - from) as u32);
            for (at, &value) in values.iter().enumerate().take(to).skip(from) {
                sink.literal(value, old.get(at).copied().unwrap_or(0.0), true);
            }
        }
    } else {
        for (source, from, to) in source_copies(values, old, stride, index) {
            operations += 1;
            words += 2;
            sink.integer(source.map_or(0, |s| s as u32 + 1));
            sink.integer((to - from) as u32);
            if source.is_none() {
                words += to - from;
                for (at, &value) in values.iter().enumerate().take(to).skip(from) {
                    sink.literal(value, old.get(at).copied().unwrap_or(0.0), true);
                }
            }
        }
    }
    (words, operations)
}

/// Short exact spans remain reusable when variable metadata shifts their addresses.
pub(super) const VARIABLE_ANCHOR_WORDS: usize = 3;

pub(super) fn compare_rows(a: &[f32], b: &[f32]) -> std::cmp::Ordering {
    a.iter()
        .map(|v| v.to_bits())
        .cmp(b.iter().map(|v| v.to_bits()))
}

/// Assemble canonical order from exact old spans and new literals. Fixed rows
/// retain their alignment; variable spans use sparse short word anchors and
/// extend wordwise. Bit comparisons never depend on entity identity or hashes.
pub(super) fn source_copies<'a>(
    values: &'a [f32],
    old: &'a [f32],
    stride: usize,
    index: &'a [u32],
) -> impl Iterator<Item = (Option<usize>, usize, usize)> + 'a {
    let anchor = if stride == 0 {
        VARIABLE_ANCHOR_WORDS
    } else {
        stride
    };
    let source = move |at: usize| {
        let row = values.get(at..at + anchor)?;
        if stride == 0
            && old
                .get(at..at + anchor)
                .is_some_and(|v| compare_rows(v, row).is_eq())
        {
            return Some(at);
        }
        let i = index
            .partition_point(|&p| compare_rows(&old[p as usize..p as usize + anchor], row).is_lt());
        let p = *index.get(i)? as usize;
        compare_rows(&old[p..p + anchor], row).is_eq().then_some(p)
    };
    let mut at = 0;
    std::iter::from_fn(move || {
        if at == values.len() {
            return None;
        }
        let start = at;
        let from = source(at);
        at += if from.is_some() {
            anchor
        } else {
            stride.max(1)
        };
        while at < values.len() {
            let step = stride.max(1);
            let follows = match from {
                Some(p) => old
                    .get(p + at - start..p + at - start + step)
                    .is_some_and(|row| compare_rows(row, &values[at..at + step]).is_eq()),
                None => source(at).is_none(),
            };
            if !follows {
                break;
            }
            at += step;
        }
        Some((from, start, at))
    })
}

/// Two scans choose a smaller payload without storing a range for every word.
pub(super) fn changed_ranges<'a>(
    values: &'a [f32],
    old: &'a [f32],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < values.len()
            && old
                .get(i)
                .is_some_and(|v| v.to_bits() == values[i].to_bits())
        {
            i += 1;
        }
        if i == values.len() {
            return None;
        }
        let start = i;
        i += 1;
        while i < values.len()
            && old
                .get(i)
                .is_none_or(|v| v.to_bits() != values[i].to_bits())
        {
            i += 1;
        }
        Some((start, i))
    })
}
