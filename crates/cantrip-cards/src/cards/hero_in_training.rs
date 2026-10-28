use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Draw a card. If you control another Hero, you gain 2 life." Another Hero is a permanent of its controller's other than
/// this one, judged as the ability resolves; the type is read through the words of this card's text (CR 612). If this creature
/// has left the battlefield by then, any Hero its controller controls is another one (CR 400.7).
fn draw_and_maybe_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    let hero = g.words(item.object).subtype(Subtype::Hero);
    let another = g.permanents_of(item.controller).into_iter().any(|id| id != item.object && g.chars(id).has_subtype(hero));
    if another {
        g.gain_life(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, draw_and_maybe_gain, "When this creature enters, draw a card. If you control another Hero, you gain 2 life.")],
    ..oracle::HERO_IN_TRAINING
};
