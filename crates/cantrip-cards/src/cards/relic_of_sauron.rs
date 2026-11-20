use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Add two mana in any combination of {U}, {B}, and/or {R}" (CR 106.1b, 605.3): each mana a color of its own choosing,
/// chosen as the mana is added.
fn add_two(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let choices = ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Black).0 | ColorSet::of(Color::Red).0);
    for color in cantrip_mechanics::choose_colors(g, asker.controller, asker.source, choices, 2)? {
        g.add_mana(asker.controller, color.mana(), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_by(add_two), "{T}: Add 2 mana in any combination of {U}, {B}, and/or {R}."),
        ability(cost("{3}, {T}"), effects::draw_then_discard::<2, 1>, "{3}, {T}: Draw two cards, then discard a card."),
    ],
    ..oracle::RELIC_OF_SAURON
};
