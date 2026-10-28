use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: flash (CR 702.8) and "can't be blocked" (CR 509.1b) are core keywords, read from the card's JSON.

pub static CARD: CardDef = CardDef { ..oracle::GHOST_SPECTRAL_SABOTEUR };
