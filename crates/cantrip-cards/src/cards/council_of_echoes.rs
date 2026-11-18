use cantrip_api::*;
use cantrip_mechanics::targets::{self, you_have_permanent_cards_in_graveyard};
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

// Flying is the core's, read from the card's JSON.
/// "Descend 4 - When this creature enters, if there are four or more permanent cards in your graveyard, return up to one target nonland
/// permanent other than this creature to its owner's hand" (CR 207.2c: descend is an ability word; 603.4: the "if" is judged as it
/// enters, so the trigger is not put on the stack with fewer, and again as it resolves).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_hand,
        "Descend 4 \u{2014} When this creature enters, if there are four or more permanent cards in your graveyard, return up to one target nonland permanent other than this creature to its owner's hand.",
    )
    .targets(&[up_to_one(targets::other_nonland_permanent_filter)])
    .condition(you_have_permanent_cards_in_graveyard::<4>)],
    ..oracle::COUNCIL_OF_ECHOES
};
