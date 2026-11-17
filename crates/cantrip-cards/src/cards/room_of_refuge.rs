use cantrip_api::*;
use cantrip_mechanics::lands::{self, enters_tapped_choosing_a_color};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This land enters tapped. As it enters, choose a color" (CR 614.1c) and "{T}: Add one mana of the chosen color"
    // (CR 605.1a, 607.2d - the choice and the mana ability are linked): the shared land template, as Sunken Citadel and
    // Valgavoth's Lair print it.
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[
        // Mana abilities first.
        lands::TAP_FOR_THE_CHOSEN_COLOR,
        ability(cost("{5}, {T}").and(SACRIFICE_THIS), effects::two_plus_one_counters_on_target, "{5}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.")
            .targets(&[targets::CREATURE])
            .sorcery_speed(),
    ],
    ..oracle::ROOM_OF_REFUGE
};
