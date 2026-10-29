use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_mutant, "When Jennika enters, create a 2/2 red Mutant creature token."),
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::JENNIKA_BAD_APPLE_BIG_SISTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
