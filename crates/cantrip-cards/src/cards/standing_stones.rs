use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{1}, {T}, Pay 1 life: Add one mana of any color." (CR 605.1a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            cost("{1}").and(TAP).and(life(1)),
            add_any_color(1),
            "{1}, {T}, Pay 1 life: Add one mana of any color.",
        ),
    ],
    ..oracle::STANDING_STONES
};
