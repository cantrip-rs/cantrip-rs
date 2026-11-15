use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add one mana of any color." (CR 106.1a, 605.1a): a mana ability, so it is in the characteristics' `abilities`.
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    ..oracle::THREE_TREE_ROOTWEAVER
};
