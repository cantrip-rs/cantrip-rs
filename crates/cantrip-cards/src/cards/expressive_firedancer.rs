use cantrip_api::*;
use cantrip_mechanics::{effects, pump, spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn. If
/// five or more mana was spent to cast that spell, this creature also gains double strike until end of turn." The trigger resolves
/// before the spell does, and the spell's mana spent (CR 601.2h) is read from it even if it has since been countered or has left the
/// stack (CR 608.2h); a spell cast without paying its mana cost has had none spent.
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump(g, item.object, item.object, 1, 1);
    g.end_sentence();
    if spent::five_or_more_mana_spent_on_the_spell(g, item) {
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::DoubleStrike);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn. If five or more mana was spent to cast that spell, this creature also gains double strike until end of turn.",
    )],
    ..oracle::EXPRESSIVE_FIREDANCER
};
