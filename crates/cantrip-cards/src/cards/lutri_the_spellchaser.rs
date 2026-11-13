use cantrip_api::*;
use cantrip_mechanics::{copy, same_name, targets};
use cantrip_oracle as oracle;

/// "Companion - Each nonland card in your starting deck has a different name" (CR 702.139a, 201.2b): no two share a name, a
/// split card sharing one with a card named for either half (CR 201.2a).
fn nonland_cards_named_apart(q: &dyn Query, deck: &StartingDeck) -> bool {
    let nonland: Vec<ObjectId> = deck.cards.iter().copied().filter(|&card| !q.chars(card).is(CardType::Land)).collect();
    nonland.iter().enumerate().all(|(i, &a)| nonland[..i].iter().all(|&b| !same_name(q, a, b)))
}

pub static CARD: CardDef = CardDef {
    companion: Some(nonland_cards_named_apart),
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST, copy::copy_target_spell, "When Lutri, the Spellchaser enters, if you cast it, copy target instant or sorcery spell you control. You may choose new targets for the copy.")
        .targets(&[targets::INSTANT_OR_SORCERY_SPELL_YOU_CONTROL])],
    ..oracle::LUTRI_THE_SPELLCHASER
};
