use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    delayed: &[ability(
        TAP.and(SACRIFICE_THIS),
        effects::draw_two_then_discard,
        "{T}, Sacrifice this artifact: Draw two cards, then discard a card.",
    )],
    statics: &[max_speed("Max speed \u{2014} {T}, Sacrifice this artifact: Draw two cards, then discard a card.")],
    ..oracle::STARTING_COLUMN
};
