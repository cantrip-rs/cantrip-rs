use cantrip_api::*;
use cantrip_mechanics::lands::{self, enters_tapped_choosing_a_color};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This land enters tapped. As it enters, choose a color" (CR 614.1c) and "{T}: Add one mana of the chosen color"
    // (CR 605.1a, 607.2d): the shared land template, as Sunken Citadel and Room of Refuge print it.
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[lands::TAP_FOR_THE_CHOSEN_COLOR],
    ..oracle::CROSSROADS_VILLAGE
};
