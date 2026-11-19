use cantrip_api::*;
use cantrip_mechanics::targets::{self, you_have_permanent_cards_in_graveyard};
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Descend 4 - When this creature enters, if there are four or more permanent cards in your graveyard, return target permanent card
/// from your graveyard to your hand" (CR 207.2c: descend is an ability word; 603.4: the "if" is judged as it enters, so the trigger is
/// not put on the stack with fewer, and again as it resolves; 115.1, 110.4: any permanent card of the controller's graveyard).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_hand,
        "Descend 4 \u{2014} When this creature enters, if there are four or more permanent cards in your graveyard, return target permanent card from your graveyard to your hand.",
    )
    .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD])
    .condition(you_have_permanent_cards_in_graveyard::<4>)],
    ..oracle::COATI_SCAVENGER
};
