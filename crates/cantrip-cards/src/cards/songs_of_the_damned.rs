use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Add {B} for each creature card in your graveyard." (CR 608.2, 106.3).
fn add_b_creature_card_in(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.state().zone(Zone::graveyard(item.controller)).iter()
        .filter(|&&id| g.chars(id).is(CardType::Creature))
        .count() as u32;
    g.add_mana(item.controller, Mana::B, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], add_b_creature_card_in),
    ..oracle::SONGS_OF_THE_DAMNED
};
