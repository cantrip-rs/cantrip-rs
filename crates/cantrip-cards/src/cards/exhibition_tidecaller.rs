use cantrip_api::*;
use cantrip_mechanics::{spent, targets, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, target player mills three cards. If five or
/// more mana was spent to cast that spell, that player mills ten cards instead." "Instead" replaces the three with ten
/// (CR 614.1a): ten cards in all, not thirteen. The spell's mana spent (CR 601.2h) is read from it, even if it has been
/// countered or has left the stack (CR 608.2h). Nothing happens if the target player is gone by now (CR 608.2b).
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    g.mill(player, if spent::five_or_more_mana_spent_on_the_spell(g, item) { 10 } else { 3 });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, target player mills three cards. If five or more mana was spent to cast that spell, that player mills ten cards instead.",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::EXHIBITION_TIDECALLER
};
