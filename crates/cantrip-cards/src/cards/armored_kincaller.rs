use cantrip_api::*;
use cantrip_mechanics::{choose, lands, select, statics, triggers};
use cantrip_oracle as oracle;

/// "You may reveal a card": up to one, since the reveal is optional (CR 701.20a).
const REVEAL_UP_TO_ONE: Selection = Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Dinosaur)]).called("Dinosaur");

/// "When this creature enters, you may reveal a Dinosaur card from your hand. If you do or if you control another Dinosaur, you
/// gain 3 life." (CR 701.20a, 608.2c, 119.1): the player chooses a Dinosaur card in their hand to reveal, or none; either the
/// reveal or another Dinosaur they control, counted as the ability resolves whether or not this creature is still there or a
/// Dinosaur (the ruling), gains the life.
fn reveal_a_dinosaur_to_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    // A Dinosaur card, read through the words of this creature's text (CR 205.3m, 612.2): nothing is asked of a hand with none.
    let hand: Vec<ObjectId> = g.state().zone(Zone::hand(you)).iter().copied().collect();
    let rule = REVEAL_UP_TO_ONE;
    let any = cantrip_api::select::first_answer(&*g, &rule, you, item.object, &hand, true).is_some_and(|taken| !taken.is_empty());
    let revealed = if any { choose::select(g, you, item.object, Purpose::Reveal, hand, rule)? } else { Vec::new() };
    g.reveal(you, &revealed);
    g.end_sentence();
    if !revealed.is_empty() || statics::you_control_another::<{ lands::sub(Subtype::Dinosaur) }>(g, &Asker::new(item.object, you)) {
        g.gain_life(you, 3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        reveal_a_dinosaur_to_gain_life,
        "When this creature enters, you may reveal a Dinosaur card from your hand. If you do or if you control another Dinosaur, you gain 3 life.",
    )],
    ..oracle::ARMORED_KINCALLER
};
