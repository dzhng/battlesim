use super::*;

#[test]
fn short_variable_spans_survive_row_growth_without_literal_resends() {
    let mut old = vec![0.0; HEADER.len()];
    let mut logical = old.clone();
    for i in 0..100u32 {
        // Growing variable metadata between unchanged three-word positions
        // leaves no retained eight-word subsequence. Exact NaN and signed
        // zero payloads also have to survive copying from the old record.
        let point = [
            f32::from_bits(0x4500_0000 + i * 7919),
            f32::from_bits(0x7fc1_0000 + i),
            -0.0,
        ];
        old.extend(point);
        old.extend(point);
        logical.push(f32::from_bits(0x4400_0000 + i));
        logical.extend(point);
        logical.extend(point);
    }
    let ends = vec![logical.len(); 9];
    let old_ends = vec![old.len(); 9];
    let mut wire = Vec::new();
    encode_delivery(&logical, &ends, &old, &old_ends, &mut wire).unwrap();
    assert!(
        wire.len() * 4 < 1400,
        "retained positions were resent: {} B",
        wire.len() * 4
    );
    let actual = codec::group(&wire, HEADER.len(), &old[HEADER.len()..]);
    assert_eq!(
        actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        logical[HEADER.len()..]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn compact_carriers_preserve_subnormals_nan_payloads_and_signed_zero() {
    let special = [
        0x0007_fc00,
        0x7f80_0001,
        0x7fc1_2345,
        0xff80_0001,
        0x8000_0000,
        0xffff_ffff,
        0x0000_0001,
        0x7f80_0000,
    ];
    let values: Vec<_> = special.into_iter().map(f32::from_bits).collect();
    let mut body = Vec::new();
    let mut writer = PackedWriter {
        out: &mut body,
        pending: 0,
        bits: 0,
    };
    pack_group(1, &values, &[], 0, &[], 0, &mut writer);
    writer.finish();
    assert!(
        body[0].is_nan(),
        "encoded carrier must actually exercise NaN bits"
    );
    let mut wire = vec![values.len() as f32, 3.0, body.len() as f32];
    wire.extend(body);
    assert_eq!(
        codec::group(&wire, 0, &[])
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        special
    );
}

#[test]
fn scattered_fixed_row_insertions_do_not_resend_the_retained_tail() {
    let row = |i: usize| {
        [
            i as f32,
            i as f32 + 0.25,
            -0.0,
            1.0,
            i as f32,
            0.0,
            0.0,
            0.0,
            f32::from_bits(0x7fc0_0001),
        ]
    };
    let old_rows: Vec<_> = (0..100).map(row).collect();
    let mut rows = old_rows.clone();
    for (at, id) in [(75, 102), (40, 101), (5, 100)] {
        rows.insert(at, row(id));
    }
    let record = |rows: &[[f32; 9]]| {
        let mut data = vec![0.0; HEADER.len()];
        data.extend(rows.iter().flatten());
        let mut ends = vec![HEADER.len(); nine_wide()];
        ends.push(data.len());
        ends.resize(GROUPS, data.len());
        (data, ends)
    };
    let (old, old_ends) = record(&old_rows);
    let (logical, ends) = record(&rows);
    let mut encoded = Vec::new();
    encode_delivery(&logical, &ends, &old, &old_ends, &mut encoded).unwrap();
    assert!(
        encoded.len() * 4 < 512,
        "three new rows must not resend 100 retained rows: {} B",
        encoded.len() * 4
    );
    assert_eq!(
        decode_group(&encoded, &old),
        rows.iter()
            .flatten()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn ground_carriers_preserve_exact_address_span_and_mark_extremes() {
    let rows = [
        0.0,
        65536.0,
        0.0,
        0.0,
        16_777_215.0,
        511.0,
        65535.0,
        16_777_215.0,
    ];
    let mut wire = Vec::new();
    let mut writer = PackedWriter {
        out: &mut wire,
        pending: 0,
        bits: 0,
    };
    pack_ground(&rows, &mut writer);
    writer.finish();
    let restored = codec::ground(&wire, 2);
    assert_eq!(
        restored.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        rows.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

mod codec {
    include!("../../tests/common/publication_codec.rs");
}
/// The first group of fixed nine-word rows, the shape these tests' rows
/// take; the groups before it travel empty.
fn nine_wide() -> usize {
    (0..GROUPS).find(|&g| fixed_row_width(g) == 9).unwrap()
}
fn decode_group(encoded: &[f32], old: &[f32]) -> Vec<u32> {
    codec::group(
        encoded,
        HEADER.len() + nine_wide() * 3,
        &old[HEADER.len()..],
    )
    .iter()
    .map(|v| v.to_bits())
    .collect()
}

#[test]
fn fixed_row_removals_and_reordering_copy_canonical_bits_without_new_literals() {
    let mut old = vec![0.0; HEADER.len()];
    for i in 0..100 {
        old.extend([
            i as f32,
            -0.0,
            0.0,
            1.0,
            i as f32,
            0.0,
            0.0,
            0.0,
            f32::from_bits(0x7fc0_0001),
        ]);
    }
    let mut old_ends = vec![HEADER.len(); nine_wide()];
    old_ends.resize(GROUPS, old.len());
    for reverse in [false, true] {
        let mut logical = vec![0.0; HEADER.len()];
        for offset in 0..80 {
            let i = if reverse { 89 - offset } else { 10 + offset };
            logical.extend_from_slice(&old[HEADER.len() + i * 9..HEADER.len() + (i + 1) * 9]);
        }
        let mut ends = vec![HEADER.len(); nine_wide()];
        ends.resize(GROUPS, logical.len());
        let mut encoded = Vec::new();
        encode_delivery(&logical, &ends, &old, &old_ends, &mut encoded).unwrap();
        assert!(
            encoded.len() * 4 < 1024,
            "retained 80-row reorder should copy source rows: {} B",
            encoded.len() * 4
        );
        assert_eq!(
            decode_group(&encoded, &old),
            logical[HEADER.len()..]
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn group_addresses_survive_earlier_group_shrink_and_keep_float_bits() {
    let words = [f32::from_bits(0x7fc0_0001), -0.0, f32::INFINITY];
    let mut previous = vec![0.0; HEADER.len()];
    previous.extend([1.0, 2.0, 3.0]);
    previous.extend(words);
    let previous_ends = [HEADER.len() + 3, previous.len()];
    let mut logical = vec![0.0; HEADER.len()];
    logical.push(1.0);
    logical.extend(words);
    let mut encoded = Vec::new();
    encode_delivery(
        &logical,
        &[HEADER.len() + 1, logical.len()],
        &previous,
        &previous_ends,
        &mut encoded,
    )
    .unwrap();
    assert_eq!(&encoded[HEADER.len()..], &[1.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
    logical[HEADER.len() + 2] = 0.0;
    logical[HEADER.len() + 1] = f32::from_bits(0x7fc0_0002);
    encode_delivery(
        &logical,
        &[HEADER.len() + 1, logical.len()],
        &previous,
        &previous_ends,
        &mut encoded,
    )
    .unwrap();
    let result = codec::group(&encoded, HEADER.len() + 3, &words);
    assert_eq!(
        result.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        logical[HEADER.len() + 1..]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
}
