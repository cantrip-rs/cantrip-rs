use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::VALIANT;
use cantrip_oracle as oracle;

/// "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, you may
/// discard a card. If you do, draw a card." (CR 603.2, 701.9, 121.1): the trigger condition is the shared valiant (only your own spells
/// and abilities, once a turn); the loot is the shared "may discard, if you do, draw" and asks nothing of an empty hand.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        VALIANT,
        |g, item| effects::may_discard_to_draw(g, item, 1),
        "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, you may discard a card. If you do, draw a card.",
    )],
    ..oracle::WHISKERQUILL_SCRIBE
};
