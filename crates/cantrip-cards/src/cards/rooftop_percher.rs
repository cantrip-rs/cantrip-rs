use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile up to two target cards from graveyards. You gain 3 life." (CR 115.1, 608.2b): the targets still in their graveyards
/// are exiled together, from any graveyards; a target that became illegal is left alone and the life is still gained as long
/// as one target remains legal (or none was chosen).
fn exile_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = (0..2).filter_map(|slot| targets::object(g, item, slot)).collect();
    g.exile_all(&cards);
    g.gain_life(item.controller, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_and_gain_life, "When this creature enters, exile up to two target cards from graveyards. You gain 3 life.")
        .targets(&[
            targets::CARD_IN_A_GRAVEYARD.optional().labelled("First card to exile (up to two, from graveyards)"),
            targets::CARD_IN_A_GRAVEYARD.optional().labelled("Second card to exile"),
        ])],
    ..oracle::ROOFTOP_PERCHER
};
