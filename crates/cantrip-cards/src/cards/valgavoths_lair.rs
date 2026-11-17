use cantrip_api::*;
use cantrip_mechanics::lands::{self, enters_tapped_choosing_a_color};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[lands::TAP_FOR_THE_CHOSEN_COLOR],
    ..oracle::VALGAVOTHS_LAIR
};
