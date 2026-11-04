use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Snow forestwalk (This creature can't be blocked as long as defending player controls a snow Forest.)" (CR 702.1).
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::SnowForestwalk]),
    ..oracle::RIME_DRYAD
};
