//! Rag Man's revealed creature remains known if a discard replacement puts it in a library (CR 701.20a, 701.9c).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn discarded(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event { Event::Discarded { card, .. } if *card == source => Fired::PLAIN, _ => None }
}
fn gain(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}
static CREATURE: CardDef = CardDef {
    name: "Rag Probe Creature", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2),
    abilities: &[triggered(when(EventMask::DISCARDED, discarded), gain, "When you discard this card, gain 3 life.").functions_in(WHERE_A_DISCARD_ENDS)],
    ..CardDef::BLANK
};
fn discarded_to_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.reason == ZoneReason::Discard && z.to.is(ZoneKind::Graveyard))
}
fn library_instead(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::library(q.obj(z.object).owner); }
}
static LENG: CardDef = CardDef {
    name: "Rag Probe Leng", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::ZoneChange, discarded_to_graveyard, library_instead, "Put discarded card on library instead")],
    ..CardDef::BLANK
};
#[test]
fn revealed_creature_discarded_into_library_retains_its_discard_trigger() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&CREATURE, &LENG])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Rag Probe Leng
Black Lotus x2
Mox Jet
Rag Man
Island x20
[Bot2 Deck]
Rag Probe Creature
Island x20
[Actions]
Bot1.Hand.Rag Probe Leng.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Rag Man.Cast
[T3 Bot1.Main1]
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Rag Man.Activate(Bot2)
Bot1.Pass
Bot2.Pass
? Stack.Count = 1
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = 23
Bot2.Graveyard.Count = 0
"#, "rag-man-revealed-library").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
