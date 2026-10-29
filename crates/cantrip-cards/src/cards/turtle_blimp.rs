use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_mutant, "When this Vehicle enters, create a 2/2 red Mutant creature token."),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::TURTLE_BLIMP
};

/// The Mutant token it makes.
pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
