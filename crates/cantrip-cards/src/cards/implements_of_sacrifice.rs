use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{1}, {T}, Sacrifice this artifact: Add two mana of any one color." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            cost("{1}").and(SACRIFICE_THIS).and(TAP),
            add_any_color(2),
            "{1}, {T}, Sacrifice this artifact: Add two mana of any one color.",
        ),
    ],
    ..oracle::IMPLEMENTS_OF_SACRIFICE
};
