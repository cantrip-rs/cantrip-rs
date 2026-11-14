use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add three mana of any one color." (CR 605.1a, 106.1): a mana ability; the color is chosen as it is activated and all
/// three mana are of it.
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(3), "{T}: Add three mana of any one color.")],
    ..oracle::GILDED_LOTUS
};
