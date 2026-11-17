use cantrip_api::*;
use cantrip_mechanics::lands::{self, chosen_color, enters_tapped_choosing_a_color};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[
        lands::TAP_FOR_THE_CHOSEN_COLOR,
        mana_ability(TAP, add_any_color(2).only_for(ManaRestriction::LAND_ABILITIES), "{T}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.")
            .colors_from(chosen_color),
    ],
    ..oracle::SUNKEN_CITADEL
};
