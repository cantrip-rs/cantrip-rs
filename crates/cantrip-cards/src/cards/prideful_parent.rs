use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Vigilance is a core keyword read from its characteristics (CR 702.20).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_cat, "When this creature enters, create a 1/1 white Cat creature token.")],
    ..oracle::PRIDEFUL_PARENT
};

pub static EXTRA: &[&CardDef] = &[&tokens::CAT];
