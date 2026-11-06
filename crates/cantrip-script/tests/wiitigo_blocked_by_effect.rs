//! Fog Patch makes unblocked attackers blocked without a blocking creature (CR 509.1h, 509.3c).
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::GameScript;

fn fog_patch(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<_> = g.state().attackers.iter().map(|a| a.id).collect();
    for id in attackers { g.become_blocked(id); }
    Ok(())
}
static FOG_PATCH: CardDef = CardDef {
    name: "Probe Fog Patch", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], fog_patch), ..CardDef::BLANK
};

#[test]
fn blocked_without_a_blocker_counts_at_the_next_upkeep() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&FOG_PATCH]))));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Mox Pearl
Mox Sapphire
Mox Jet
Wiitigo
Forest x20
[Bot2 Deck]
Probe Fog Patch
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Wiitigo.Cast
[T3 Bot1.Main1]
? Bot1.Battlefield.Wiitigo.Counters.PlusOnePlusOne = 5
Bot1.Attack(Wiitigo > Bot2)
[T3 Bot1.Blockers]
Bot2.Hand.Probe Fog Patch.Cast
[T5 Bot1.Main1]
[Expect]
Bot1.Battlefield.Wiitigo.Counters.PlusOnePlusOne = 6
"#, "wiitigo-blocked-by-effect").unwrap();
    cantrip_script::run_on(&script, table).map_err(|f| f.report).expect("blocked without a blocker counts");
}
