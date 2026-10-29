use renderer_core::bundle::Library;

fn empty_bundle() -> Vec<u8> {
    let mut header = serde_json::to_vec(&serde_json::json!({
        "content": {"kind":"static", "states":[], "materials":[]},
        "views": []
    }))
    .unwrap();
    header.resize(header.len().div_ceil(4) * 4, b' ');
    let mut bytes = Vec::new();
    for word in [0x5441_4742_u32, 4, header.len() as u32, 0] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(header);
    bytes
}

#[test]
fn shared_texture_payload_is_retained_once_across_bundles_and_catalog_replacement() {
    let bytes = empty_bundle();
    let pixels: Vec<u8> = b"rgba8unorm-srgb 2x2\n"
        .iter()
        .copied()
        .chain(0..20)
        .collect();
    let mut library = Library::default();
    library
        .insert_bundle("house", &bytes, &[("same-content", &pixels)])
        .unwrap();
    library
        .insert_bundle("vehicle", &bytes, &[("same-content", &pixels)])
        .unwrap();
    assert_eq!(library.payload_bytes(), 20);
    let mut replacement = library.empty_generation();
    assert!(replacement.bundle("house").is_err());
    replacement
        .insert_bundle("new-vehicle", &bytes, &[("same-content", &pixels)])
        .unwrap();
    replacement.retain(|name| name == "new-vehicle");
    library.replace_with(replacement);
    library.retain(|name| name == "new-vehicle");
    assert_eq!(library.payload_bytes(), 20);
    let bundle = library.bundle("new-vehicle").unwrap();
    assert_eq!(bundle.textures[0].levels[0], (0..16).collect::<Vec<u8>>());
    assert_eq!(bundle.textures[0].levels[1], (16..20).collect::<Vec<u8>>());
    assert!(bundle.textures[0].srgb);
    drop(bundle);
    library.retain(|_| false);
    assert_eq!(library.payload_bytes(), 0);
}
