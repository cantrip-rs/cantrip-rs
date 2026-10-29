use cantrip_api::*;
use cantrip_mechanics::{effects, spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent.
/// If five or more mana was spent to cast that spell, this creature deals 3 damage to each opponent instead." The spell's mana spent
/// (CR 601.2h) is read from it even if it has since been countered or has left the stack (CR 608.2h); a spell cast without paying its
/// mana cost has had none spent. The damage is dealt by this creature as it last existed if it has left the battlefield (CR 113.7a).
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let damage = if spent::five_or_more_mana_spent_on_the_spell(g, item) { 3 } else { 1 };
    effects::damage_each_opponent(g, item, damage);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent. If five or more mana was spent to cast that spell, this creature deals 3 damage to each opponent instead.",
    )],
    ..oracle::THUNDERDRUM_SOLOIST
};
