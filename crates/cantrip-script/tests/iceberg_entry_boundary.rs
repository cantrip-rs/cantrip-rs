//! An X entry preserves counter totals across the former 16-bit ceiling (upstream PR #2406).
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

// Deliberately seed the entry state: a boundary probe, not a claim that this artificial card is printed.
fn seed_entry(g: &mut dyn Rules, entry: &mut Entering) -> Result<(), Suspend> {
    entry.counters[CounterKind::Ice as usize] = i32::from(i16::MAX);
    cantrip_mechanics::effects::enters_with_x_counters::<{CounterKind::Ice as usize}>(g, entry)
}
static PROBE: CardDef = CardDef {
    name: "Probe Ice Entry", oracle: true, has_cost: true, cost: mana("{X}"),
    types: TypeSet::of(&[CardType::Enchantment]), enters: Some(seed_entry), ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&PROBE])))
}
#[test]
fn x_added_to_entry_counters_crosses_the_former_i16_ceiling() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Pearl
Probe Ice Entry
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Probe Ice Entry.Cast(X: 1)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Probe Ice Entry.Counters.Ice = 32768
"#, "iceberg-entry-boundary").unwrap();
    if let Err(failure) = run_on(&script, table()) { panic!("{}", failure.report); }
}
