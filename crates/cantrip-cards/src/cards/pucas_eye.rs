use cantrip_api::*;
use cantrip_mechanics::{choose_color, effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Draw a card, then choose a color. This artifact becomes the chosen color." (CR 608.2c, 105.1, 613.1e): in that order, and
/// the color is the artifact's for as long as it stays on the battlefield, replacing its colorlessness and not adding to it.
fn draw_then_become_a_chosen_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    let color = choose_color(g, item.controller, item.object)?;
    let words = g.words(item.object);
    g.add_effect(item.object, item.object, EffectKind::SetColors(words.colors(ColorSet::of(color))), Duration::Indefinite);
    Ok(())
}

/// "Activate only if there are five colors among permanents you control" (CR 602.5b, 105.2): the colors of every permanent its
/// controller controls, read through the layers as the ability is activated.
fn five_colors_among_your_permanents(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().fold(ColorSet::NONE, |all, id| ColorSet(all.0 | q.chars(id).colors.0)).iter().count() == 5
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, draw_then_become_a_chosen_color, "When this artifact enters, draw a card, then choose a color. This artifact becomes the chosen color."),
        ability(
            cost("{3}, {T}"),
            |g, item| effects::draw(g, item, 1),
            "{3}, {T}: Draw a card. Activate only if there are five colors among permanents you control.",
        )
        .condition(five_colors_among_your_permanents),
    ],
    ..oracle::PUCAS_EYE
};
