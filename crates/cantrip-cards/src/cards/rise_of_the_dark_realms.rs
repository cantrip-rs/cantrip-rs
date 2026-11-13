use cantrip_api::*;
use cantrip_mechanics::graveyards;
use cantrip_oracle as oracle;

/// "Put all creature cards from all graveyards onto the battlefield under your control." (CR 404.1, 110.2a, 101.4): every creature
/// card in every graveyard as the spell resolves, an opponent's included, enters together under the spell's controller's control,
/// so each sees the others enter (CR 603.6a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<(ObjectId, PlayerId)> =
        graveyards::cards_in_graveyards(g.state()).filter(|&card| g.chars(card).is(CardType::Creature)).map(|card| (card, you)).collect();
    g.put_all_onto_battlefield(&cards)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::RISE_OF_THE_DARK_REALMS };
