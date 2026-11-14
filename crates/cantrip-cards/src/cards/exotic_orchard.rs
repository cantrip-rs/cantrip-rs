use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::mana::opponents_land_colors;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color that a land an opponent controls could produce.")
        .colors_from(opponents_land_colors)],
    ..oracle::EXOTIC_ORCHARD
};
