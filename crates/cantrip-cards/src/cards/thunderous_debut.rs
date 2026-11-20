use cantrip_api::*;
use cantrip_mechanics::{bargain, choose, select, top_of_library};
use cantrip_oracle as oracle;

/// "Look at the top twenty cards of your library. You may reveal up to two creature cards from among them. If this spell was
/// bargained, put the revealed cards onto the battlefield. Otherwise, put the revealed cards into your hand. Then shuffle."
/// (CR 701.20a, 702.166b, 701.24): the cards looked at stay in the library, a library of fewer cards shows what it has
/// (CR 609.3), and the revealed creatures enter together under your control (CR 110.2a) or go to the hand revealed. The
/// library is shuffled whatever was revealed.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 20);
    g.look_at(you, &looked);
    let revealed = choose::select(g, you, item.object, Purpose::Reveal, looked, select::creature_cards(select::up_to(2)))?;
    if !revealed.is_empty() {
        g.reveal(you, &revealed);
        if bargain::bargained(item) {
            let entering: Vec<_> = revealed.into_iter().map(|card| (card, you, false, you)).collect();
            g.put_all_onto_battlefield_by(&entering, &[])?;
        } else {
            g.return_to_hand_revealed(&revealed);
        }
    }
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    optional_cost: Some(bargain::OPTIONAL_COST),
    labels: bargain::LABELS,
    ..oracle::THUNDEROUS_DEBUT
};
