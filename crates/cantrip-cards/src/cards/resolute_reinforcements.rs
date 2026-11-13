use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flash is a core keyword read from its characteristics (CR 702.8).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_soldier, "When this creature enters, create a 1/1 white Soldier creature token.")],
    ..oracle::RESOLUTE_REINFORCEMENTS
};

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
