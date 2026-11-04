use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever you scry or surveil, draw a card" (CR 701.22d, 701.25d, 121.1): each scry and each surveil of its controller's
/// is one trigger, resolving after the scry or surveil is complete.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_SCRY_OR_SURVEIL, |g, item| effects::draw(g, item, 1), "Whenever you scry or surveil, draw a card.")],
    ..oracle::MATOYA_ARCHON_ELDER
};
