use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

/// "Flying" and "vigilance" are core keywords (CR 702.9, 702.20). "At the beginning of your end step, if you haven't cast a
/// spell from your hand this turn, draw a card." (CR 603.4): the intervening "if" is checked as the step begins and again as
/// the ability resolves, so a spell cast from hand in between leaves the draw undone.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_END_STEP, |g, item| effects::draw(g, item, 1), "At the beginning of your end step, if you haven't cast a spell from your hand this turn, draw a card.")
        .condition(history::you_havent_cast_a_spell_from_your_hand)],
    ..oracle::JEM_LIGHTFOOTE_SKY_EXPLORER
};
