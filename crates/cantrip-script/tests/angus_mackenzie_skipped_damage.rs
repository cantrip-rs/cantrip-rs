//! An extra combat made before attackers are declared: the first combat has no attackers, so its damage step is skipped.
//! Savage Beating can create an additional combat during combat; this zero-cost test spell isolates that effect (CR 506.8e, 508.8).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn extra_combat(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_combat_phase();
    Ok(())
}
static EXTRA: CardDef = CardDef {
    name: "Probe Extra Combat", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], extra_combat), ..CardDef::BLANK
};

#[test]
fn skipped_first_damage_step_still_closes_activation_window() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&EXTRA])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Angus Mackenzie
Mox Emerald
Mox Pearl
Mox Sapphire
Probe Extra Combat
Island x20
[Bot2 Deck]
Island x25
[Actions]
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Angus Mackenzie.Cast
Bot1.Pass
[T3 Bot1.BeginCombat]
Bot1.Hand.Probe Extra Combat.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.EndCombat]
[T3 Bot1.BeginCombat]
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
! Bot1.Battlefield.Angus Mackenzie.Activate
? Bot1.Battlefield.Angus Mackenzie.Tapped = false
[Expect]
Bot1.Battlefield.Angus Mackenzie.Tapped = false
"#, "angus-skipped-damage").expect("scenario parses");
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
