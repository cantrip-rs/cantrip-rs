use cantrip_api::*;
use cantrip_oracle as oracle;

// Characteristics only: assist is the core's, read from the card's JSON ("assist": true), and the rest of its text is
// the core keywords (Trample, haste), so nothing is left to write.
pub static CARD: CardDef = CardDef { ..oracle::MAGMA_HELLION };
