use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "You may cast spells as though they had flash" (CR 702.8a, 601.3): every spell its controller casts, from whatever zone, each
/// judged as the face that would be cast (`statics::spells_you_cast`). Flash and flying are core keywords.
pub static CARD: CardDef = CardDef { flash_rules: &[statics::spells_you_cast], ..oracle::HIGH_FAE_TRICKSTER };
