//! A copied entry preserves counter totals above the former 16-bit ceiling (upstream PR #2406).
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

// Test-only acceleration of a state reachable by repeated proliferation.
fn fill(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let Some(id) = cantrip_mechanics::targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(id, CounterKind::Javelin, i32::from(i16::MAX - 1));
    Ok(())
}
static SEED: CardDef = CardDef {
    name: "Probe Counter Seed", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{0}"), fill, "Put javelin counters on target creature.")
        .targets(&[cantrip_mechanics::targets::CREATURE])],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SEED])))
}
#[test]
fn copied_entry_keeps_javelin_counters_above_the_former_i16_ceiling() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Pearl
Icatian Javelineers
Probe Counter Seed
Black Lotus
Black Lotus
Dominion Saboteur
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Icatian Javelineers.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Probe Counter Seed.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Probe Counter Seed.Activate(Icatian Javelineers)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Icatian Javelineers.Counters.Javelin = 32767
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: Blue)
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Dominion Saboteur.Cast
Bot1.Pass
Bot2.Pass
Bot1.Accept
Bot1.Target(Icatian Javelineers)
[Expect]
Bot1.Battlefield.Icatian Javelineers#1.Counters.Javelin = 32767
Bot1.Battlefield.Icatian Javelineers#2.Counters.Javelin = 32768
"#, "icatian-javelineers-copied-entry").unwrap();
    if let Err(failure) = run_on(&script, table()) { panic!("{}", failure.report); }
}
