use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

// "This ability triggers only once each turn" (CR 701.22a, 701.25a, 603.2h): the once-a-turn limit applies however many
// scrys or surveils happen, and a scry 0 or surveil 0 is no event at all.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_SCRY_OR_SURVEIL,
        tokens::create_a_thopter,
        "Whenever you scry or surveil, create a 1/1 colorless Thopter artifact creature token with flying. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::SAHEELI_CONSUL_OF_OVERSIGHT
};

pub static EXTRA: &[&CardDef] = &[&tokens::THOPTER];
