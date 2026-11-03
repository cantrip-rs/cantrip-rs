//! Brown Ouphe cannot target players or permanents, even with an artifact ability available.
use std::sync::OnceLock;
use cantrip_engine::CardTable;
use cantrip_script::{GameScript, run_on};

#[test]
fn only_abilities_are_legal_targets() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()));
    let script = GameScript::parse(
        include_str!("../../../scenarios/brown-ouphe-refuses-nonartifact-ability.cantrip"),
        "brown-ouphe-refuses-nonartifact-ability",
    ).unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
