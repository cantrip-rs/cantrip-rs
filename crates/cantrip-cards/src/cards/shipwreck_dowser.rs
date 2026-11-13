use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// Prowess (CR 702.108a) is a real triggered ability. "When this creature enters, return target instant or sorcery card from your
/// graveyard to your hand." (CR 603.6a, 608.2b): the target is chosen as the trigger goes on the stack, so only a card already in
/// the graveyard is one, and one that has left it by resolution is illegal.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return target instant or sorcery card from your graveyard to your hand.")
            .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::SHIPWRECK_DOWSER
};
