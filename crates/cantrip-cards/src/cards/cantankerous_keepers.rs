use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{affinity, lands};
use cantrip_oracle as oracle;

/// "Mill four cards, then put all Elf cards from among them into your hand." (CR 701.17a, 701.17c, 608.2c): the cards milled are
/// the ones it takes from, as they are where they went (a graveyard, or exile by a replacement effect), every Elf card of them,
/// together. The type is read through the words of this card's text (CR 612), and a card is an Elf as the card in that zone is
/// (a changeling is every creature type, CR 702.73a).
fn mill_then_take_the_elves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let milled = g.mill(you, 4);
    g.end_sentence();
    let elves: Vec<ObjectId> = milled.into_iter().filter(|&card| g.chars(card).has_subtype(elf)).collect();
    g.return_to_hand(&elves);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_creature_type::<{ lands::sub(Subtype::Elf) }>],
    abilities: &[triggered(
        THIS_ENTERS,
        mill_then_take_the_elves,
        "When this creature enters, mill four cards, then put all Elf cards from among them into your hand.",
    )],
    ..oracle::CANTANKEROUS_KEEPERS
};
