use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, draw a card. Then discard a card unless five or
/// more mana was spent to cast that spell." The spell's mana spent (CR 601.2h) is read from it, even if it has been countered
/// or has left the stack (CR 608.2h). The discard is the controller's choice (CR 701.9a) and is skipped with an empty hand.
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    if !spent::five_or_more_mana_spent_on_the_spell(g, item) {
        discard_chosen(g, item.controller, item.object, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, draw a card. Then discard a card unless five or more mana was spent to cast that spell.",
    )],
    ..oracle::MUSE_SEEKER
};
