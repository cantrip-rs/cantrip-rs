use cantrip_api::*;
use cantrip_mechanics::lands::{self, enters_tapped_choosing_a_color};
use cantrip_oracle as oracle;

/// "For each color among monocolored permanents you control, add one mana of that color" (CR 106.5, 105.2, 105.4): the colors of the
/// permanents the controller controls that are exactly one color as the ability resolves, each counted once however many
/// permanents have it, a multicolored or colorless permanent not at all, the mana added together. A mana ability's `add_by`.
fn add_one_of_each_color_among_your_monocolored_permanents(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let mut colors = ColorSet::NONE;
    for id in g.permanents_of(asker.controller) {
        let own = g.chars(id).colors;
        if own.is_monocolored() {
            colors = ColorSet(colors.0 | own.0);
        }
    }
    for color in colors.iter() {
        g.add_mana(asker.controller, color.mana(), 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_choosing_a_color),
    abilities: &[
        lands::TAP_FOR_THE_CHOSEN_COLOR,
        mana_ability(
            cost("{1}, {T}"),
            add_by(add_one_of_each_color_among_your_monocolored_permanents),
            "{1}, {T}: For each color among monocolored permanents you control, add one mana of that color.",
        ),
    ],
    ..oracle::TARNATION_VISTA
};
