use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Flying" (CR 702.9) is the card's own keyword. "Whenever this creature attacks, mill a card" (CR 701.17a): the controller's
/// own library.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, effects::you_mill::<1>, "Whenever this creature attacks, mill a card.")],
    ..oracle::SCREAMING_PHANTOM
};
