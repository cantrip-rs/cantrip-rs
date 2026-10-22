use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(DISCARD_HAND.and(SACRIFICE_THIS), add_any_color(3), "Discard your hand, Sacrifice this artifact: Add three mana of any one color. Activate only as an instant.")
        .only_as_an_instant()],
    ..oracle::LIONS_EYE_DIAMOND
};
