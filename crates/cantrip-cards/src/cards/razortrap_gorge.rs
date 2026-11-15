use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(lands::enters_tapped_unless_a_player_has_life_at_most::<13>),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Black, Color::Red]), "{T}: Add {B} or {R}.")],
    ..oracle::RAZORTRAP_GORGE
};
