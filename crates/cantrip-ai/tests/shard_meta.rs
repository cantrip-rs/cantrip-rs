//! A game's meta as the shards hold it (`cantrip_ai::shard::GameMeta`).

/// A per-decision value written as JSON's `null` (serde_json writes a non-finite f32 so; self-play games before
/// 2026-10-28 can hold one) reads as a NaN, not a refusal of the whole game.
#[test]
fn a_null_value_reads_as_not_a_number() {
    let meta: cantrip_ai::shard::GameMeta = serde_json::from_str(
        r#"{"seed":1,"names":[],"decks":[],"modes":[],"ended":"won","winner":null,"turns":3,"answers":9,"root_values":[[4,0.5],[7,null]]}"#,
    )
    .expect("a game with a null root value reads");
    assert_eq!(meta.root_values[0], (4, 0.5));
    assert!(meta.root_values[1].1.is_nan());
}
