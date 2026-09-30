use contract::identity::Seed;

#[test]
fn a_seed_roundtrips_above_javascript_precision_and_has_one_decimal_identity() {
    for decimal in ["0", "9007199254740993", "18446744073709551615"] {
        let input = format!("\"{decimal}\"");
        let seed: Seed = serde_json::from_str(&input).unwrap();
        assert_eq!(seed.value().to_string(), decimal);
        assert_eq!(serde_json::to_string(&seed).unwrap(), input);
    }
    for input in [
        "\"0001\"",
        "\"+1\"",
        "\" 1\"",
        "\"1e3\"",
        "\"18446744073709551616\"",
        "9007199254740993",
    ] {
        assert!(serde_json::from_str::<Seed>(input).is_err(), "{input}");
    }
}
