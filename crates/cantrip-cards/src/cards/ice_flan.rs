use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, typecycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::tap_and_stun::<1>,
            "When this creature enters, tap target artifact or creature an opponent controls. Put a stun counter on it.",
        )
        .targets(&[targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS]),
        typecycling::ability(Subtype::Island, mana("{2}"), "Islandcycling {2} ({2}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::ICE_FLAN
};
