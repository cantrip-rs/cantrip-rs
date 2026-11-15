use cantrip_api::*;
use cantrip_mechanics::{choose, effects, gift, select};
use cantrip_oracle as oracle;

/// "Destroy all creatures. If the gift was promised, return a creature card put into your graveyard this way to the battlefield
/// under your control." (CR 701.8a, 101.4, 608.2c, 702.174j): the creatures on the battlefield as it resolves are destroyed
/// together (the gift card is drawn before, by the rules); then, promised, you choose one of the creature cards that really went
/// to your graveyard this way, as it resolves (the ruling): a token is no card (CR 111.7), an indestructible or regenerated creature
/// did not go, and an opponent's creature card went to their graveyard, not yours. The choice is asked only when there is a card.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures = effects::creatures_on_the_battlefield(g);
    let destroyed = g.destroy_all(&creatures);
    if item.gift.is_none() {
        return Ok(());
    }
    let cards: Vec<ObjectId> = destroyed.into_objects().into_iter().filter(|&id| !g.obj(id).token && g.obj(id).zone == Zone::graveyard(you)).collect();
    if let Some(&card) = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, cards, select::creature_cards(select::exactly(1)))?.first() {
        g.put_onto_battlefield(card, you, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), gift: Some(gift::CARD), ..oracle::STARFALL_INVOCATION };
