use cantrip_api::*;
use cantrip_mechanics::tokens::{self, MUTAGEN};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever Genghis Frog or another Mutant you control enters" (CR 603.6a): a Mutant under your control as it entered, this Frog
/// included (it is a Mutant itself), a token too.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Mutant) }>),
        tokens::create_mutagen,
        "Whenever Genghis Frog or another Mutant you control enters, create a Mutagen token.",
    )],
    ..oracle::GENGHIS_FROG
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
