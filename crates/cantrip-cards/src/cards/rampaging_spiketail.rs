use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::target_creature_gets_and_gains_indestructible::<2, 0>,
            "When this creature enters, target creature you control gets +2/+0 and gains indestructible until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        typecycling::ability(Subtype::Swamp, mana("{2}"), "Swampcycling {2} ({2}, Discard this card: Search your library for a Swamp card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::RAMPAGING_SPIKETAIL
};
