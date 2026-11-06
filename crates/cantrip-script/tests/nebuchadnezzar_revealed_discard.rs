//! A card revealed by Nebuchadnezzar stays revealed when a discard replacement puts it in a library (CR 701.20a, 701.9c).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn this_discarded(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event { Event::Discarded { card, .. } if *card == source => Fired::PLAIN, _ => None }
}
fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3); Ok(())
}
static BONES: CardDef = CardDef {
    name: "Probe Bones", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    abilities: &[triggered(when(EventMask::DISCARDED, this_discarded), gain_three, "When you discard this card, you gain 3 life.").functions_in(WHERE_A_DISCARD_ENDS)],
    ..CardDef::BLANK
};
fn discarded(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.reason == ZoneReason::Discard && z.to.is(ZoneKind::Graveyard))
}
fn to_library(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::library(q.obj(z.object).owner); }
}
static LENG: CardDef = CardDef {
    name: "Probe Leng", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::ZoneChange, discarded, to_library, "Put discarded cards into their owners' libraries")],
    ..CardDef::BLANK
};
#[test]
fn revealed_discard_remains_known_in_library() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BONES, &LENG])));
    let text = r#"
[Bot1 Deck]
Mox Sapphire
Mox Jet
Mox Pearl
Mox Ruby
Mox Emerald
Nebuchadnezzar
Probe Leng
Black Lotus
Island x20
[Bot2 Deck]
Probe Bones
Island x25
[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Emerald.Cast
Bot1.Hand.Probe Leng.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Nebuchadnezzar.Cast
Bot1.Pass
[T3 Bot1.Main1]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Nebuchadnezzar.Activate(Bot2; X: 8)
Bot1.Pass
Bot2.Pass
Bot1.Name(Probe Bones)
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = 23
"#;
    let script = GameScript::parse(text, "nebuchadnezzar-revealed-discard").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
