use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

/// Defender is in the content file; "When this creature dies, create a Treasure token" makes it for the ability's controller,
/// who controlled the creature as it died (CR 603.3a, 603.10a).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, tokens::create_a_treasure, "When this creature dies, create a Treasure token.")],
    ..oracle::GLEAMING_BARRIER
};
