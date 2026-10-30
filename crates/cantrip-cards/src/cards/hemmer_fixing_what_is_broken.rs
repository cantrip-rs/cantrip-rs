use cantrip_api::*;
use cantrip_mechanics::{effects, statics, triggers::THIS_BECOMES_TAPPED};
use cantrip_oracle as oracle;

/// "During your turn, Spacecraft you control have hexproof" (CR 611.3a, 613.1f, 702.11b).
fn your_spacecraft_on_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    let you = q.state().obj(source).controller;
    q.state().turn.active == you
        && q.state().obj(affected).controller == you
        && chars.has_subtype(q.words(source).subtype(Subtype::Spacecraft))
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        your_spacecraft_on_your_turn,
        statics::have_hexproof,
        "During your turn, Spacecraft you control have hexproof.",
    )],
    // CR 603.2e, 121.1, 701.9: only becoming tapped triggers; draw precedes discard.
    abilities: &[triggered(THIS_BECOMES_TAPPED, effects::loot, "Whenever Hemmer becomes tapped, draw a card, then discard a card.")],
    ..oracle::HEMMER_FIXING_WHAT_IS_BROKEN
};
