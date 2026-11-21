use cantrip_api::*;
use cantrip_mechanics::{delayed, firebending, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(lands::enters_tapped_unless_you_control_basic_land),
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        ability(
            cost("{1}{R}, {T}"),
            delayed::first_target_gains_its_delayed_ability_until_end_of_turn,
            "{1}{R}, {T}: Target creature you control gains firebending 4 until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    // The granted ability (the card's delayed ability 0).
    delayed: &[firebending::trigger::<4>("Firebending 4 (Whenever this creature attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)")],
    ..oracle::FIRE_NATION_PALACE
};
