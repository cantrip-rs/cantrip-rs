use cantrip_api::*;
use cantrip_mechanics::lands::{chosen_color, enters_tapped_choosing_a_color};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This land enters tapped. As it enters, choose a color" (CR 614.1c) and "{T}: Add one mana of the chosen color"
    // (CR 605.1a, 607.2d): the shared land template, as Crossroads Village prints it. "Cycling {3}" (CR 702.29a).
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of the chosen color.").colors_from(chosen_color),
        triggers::cycling(mana("{3}"), "Cycling {3} ({3}, Discard this card: Draw a card.)"),
    ],
    ..oracle::NIGHT_MARKET
};
