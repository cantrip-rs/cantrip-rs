use cantrip_api::*;
use cantrip_mechanics::{library, spell_mastery};
use cantrip_oracle as oracle;

/// "Search your library for a card, put that card into your hand, then shuffle. Spell mastery - If there are two or more
/// instant and/or sorcery cards in your graveyard, add {B}{B}{B}." (CR 701.23a, 207.2c, 608.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::search_for_a_card_to_hand(g, item)?;
    g.end_sentence();
    if spell_mastery(g, item.controller) {
        g.add_mana(item.controller, Mana::B, 3);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::DARK_PETITION };
