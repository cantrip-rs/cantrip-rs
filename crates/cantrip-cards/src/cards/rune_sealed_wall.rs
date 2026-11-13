use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

// Defender is the core's, read from the card's JSON; surveil is the shared keyword action (CR 701.25a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, |g, item| effects::surveil(g, item, 1), "{T}: Surveil 1.")],
    ..oracle::RUNE_SEALED_WALL
};
