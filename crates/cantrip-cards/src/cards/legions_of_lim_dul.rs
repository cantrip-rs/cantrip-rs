use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Snow swampwalk (This creature can't be blocked as long as defending player controls a snow Swamp.)" (CR 702.1).
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::SnowSwampwalk]),
    ..oracle::LEGIONS_OF_LIM_D_L
};
