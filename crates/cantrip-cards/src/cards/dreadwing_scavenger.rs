use cantrip_api::*;
use cantrip_mechanics::statics::{have_deathtouch, itself_with_threshold, plus_pt};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const THRESHOLD: &str = "Threshold \u{2014} This creature gets +1/+1 and has deathtouch as long as there are seven or more cards in your graveyard.";

/// One ability with two trigger events (CR 603.2): "enters or attacks" loots (CR 121.1, 701.9) on each. Threshold (CR 207.2c) is
/// one sentence of two static effects, +1/+1 in the power and toughness layer (CR 613.4c) and deathtouch in the ability layer
/// (CR 613.1f), both for as long as the graveyard holds seven cards.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS_OR_ATTACKS, effects::loot, "Whenever this creature enters or attacks, draw a card, then discard a card.")],
    statics: &[
        static_ability(Layer::Ability, itself_with_threshold, have_deathtouch, THRESHOLD),
        static_ability(Layer::PtModify, itself_with_threshold, plus_pt::<1, 1>, THRESHOLD).continues(),
    ],
    ..oracle::DREADWING_SCAVENGER
};
