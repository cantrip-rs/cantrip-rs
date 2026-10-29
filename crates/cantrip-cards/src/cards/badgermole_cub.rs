use cantrip_api::*;
use cantrip_mechanics::{earthbend, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        earthbend::enters::<1>("When Badgermole Cub enters, earthbend 1."),
        triggered(
            when(EventMask::TAPPED, mana::you_tap_a_creature_for_mana),
            mana::that_player_adds_an_additional::<{ Mana::G as u8 }>,
            "Whenever you tap a creature for mana, add an additional {G}.",
        )
        .mana_ability(TriggeredMana::One(Mana::G)),
    ],
    delayed: &[earthbend::RETURN],
    ..oracle::BADGERMOLE_CUB
};
