use cantrip_api::*;
use cantrip_mechanics::replacements::PLUS_ONE_COUNTERS_ON_YOUR_CREATURES;
use cantrip_mechanics::tokens::MUTAGEN;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_mutagen, "When Michelangelo enters, create a Mutagen token.")],
    replacements: &[PLUS_ONE_COUNTERS_ON_YOUR_CREATURES],
    ..oracle::MICHELANGELO_WEIRDNESS_TO_11
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
