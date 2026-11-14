//! Funeral Room // Awakening Hall, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Awakening Hall: "When you unlock this door, return all creature cards from your graveyard to the battlefield." Every
/// creature card in the graveyard as this resolves enters together (CR 400.7), under its owner's control (CR 110.2a).
fn hall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<(ObjectId, PlayerId)> = g
        .state()
        .zone(Zone::graveyard(you))
        .iter()
        .copied()
        .filter(|&card| g.chars(card).is(CardType::Creature))
        .map(|card| (card, you))
        .collect();
    g.put_all_onto_battlefield(&creatures)?;
    Ok(())
}

pub static FUNERAL_ROOM: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_DIES,
        effects::each_opponent_loses_1_life_you_gain_1,
        "Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life.",
    )],
    ..oracle::FUNERAL_ROOM
};

pub static AWAKENING_HALL: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_UNLOCK_THIS_RIGHT_DOOR, hall, "When you unlock this door, return all creature cards from your graveyard to the battlefield.")],
    ..oracle::AWAKENING_HALL
};

pub static CARD: CardDef = CardDef { split: Some((&FUNERAL_ROOM, &AWAKENING_HALL)), ..oracle::FUNERAL_ROOM_AWAKENING_HALL };

pub static EXTRA: &[&CardDef] = &[&FUNERAL_ROOM, &AWAKENING_HALL];
