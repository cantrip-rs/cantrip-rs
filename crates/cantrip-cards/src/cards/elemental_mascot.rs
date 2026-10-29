use cantrip_api::*;
use cantrip_mechanics::{library, pump, spent, triggers};
use cantrip_oracle as oracle;

/// Opus (an ability word, CR 207.2c): "Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn. If
/// five or more mana was spent to cast that spell, exile the top card of your library. You may play that card until the end of your
/// next turn." The spell's mana spent (CR 601.2h) is read from it even if it has since been countered or has left the stack (CR
/// 608.2h); a spell cast without paying its mana cost has had none spent. The pump happens whatever was spent, and the exile only at
/// five or more. The exiled card is a new object (CR 400.7) that may be played, a land under the ordinary land-play rules.
fn opus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump(g, item.object, item.object, 1, 0);
    g.end_sentence();
    if spent::five_or_more_mana_spent_on_the_spell(g, item) {
        library::exile_top_card_play_until_end_of_next_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        opus,
        "Opus \u{2014} Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn. If five or more mana was spent to cast that spell, exile the top card of your library. You may play that card until the end of your next turn.",
    )],
    ..oracle::ELEMENTAL_MASCOT
};
