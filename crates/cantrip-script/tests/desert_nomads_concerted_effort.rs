//! The landwalk clause of Concerted Effort, with the real Desert Nomads.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::triggers;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

// Fixture for the printed Concerted Effort's landwalk clause. The position has none of its other listed abilities.
fn share_landwalk(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<_> = g.permanents_of(item.controller).into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let walks = creatures.iter().fold(Keywords::NONE, |set, &id| Keywords(set.0 | (g.chars(id).keywords.0 & Keywords::LANDWALKS.0)));
    g.one_instruction(&mut |g| {
        for &id in &creatures { g.add_effect(item.object, id, EffectKind::AddKeywords(walks), Duration::EndOfTurn); }
        Ok(())
    })
}
static CONCERTED: CardDef = CardDef {
    name: "Concerted Effort", oracle: true, has_cost: true,
    cost: cost("{2}{W}{W}").mana, types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(triggers::EACH_UPKEEP, share_landwalk, "At the beginning of each upkeep, creatures you control gain landwalk until end of turn if a creature you control has landwalk.")],
    ..CardDef::BLANK
};
#[test]
fn printed_desertwalk_is_shared_and_changes_blocking() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| { let mut cards = cantrip_cards::all(); cards.push(&CONCERTED); CardTable::new(cards) });
    let text = "[Bot1 Deck]\nDesert Nomads\nBlack Lotus\nBlack Lotus\nMox Pearl\nConcerted Effort\nMemnite\nIsland x20\n[Bot2 Deck]\nScavenger Grounds\nBlack Lotus\nAgeless Guardian\nIsland x20\n[Actions]\nBot1.Hand.Black Lotus.Cast\nBot1.Hand.Black Lotus.Cast\nBot1.Hand.Mox Pearl.Cast\nBot1.Hand.Memnite.Cast\nBot1.Battlefield.Black Lotus#1.Activate(Color: Red)\nBot1.Hand.Desert Nomads.Cast\nBot1.Battlefield.Black Lotus.Activate(Color: White)\nBot1.Battlefield.Mox Pearl.Activate\nBot1.Hand.Concerted Effort.Cast\n[T2 Bot2.Main1]\nBot2.Hand.Scavenger Grounds.Play\nBot2.Hand.Black Lotus.Cast\nBot2.Battlefield.Black Lotus.Activate(Color: White)\nBot2.Hand.Ageless Guardian.Cast\n[T3 Bot1.DeclareAttackers]\nBot1.Attack(Memnite > Bot2)\n! Bot2.Block(Ageless Guardian > Memnite)\n[T3 Bot1.Main2]\n[Expect]\nBot1.Battlefield.Desert Nomads.Keywords = Desertwalk\nBot1.Battlefield.Memnite.Keywords = Desertwalk\nBot2.Life = 19\n";
    let script = GameScript::parse(text, "desert-nomads-concerted-effort").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
