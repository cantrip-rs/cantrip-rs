use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// "You may exile one or more creature cards from your graveyard. When you do, ..." (CR 608.2d, 603.12): the cards are chosen
/// and exiled together; choosing none is not doing it, so the reflexive ability does not trigger. It triggers counting them, so
/// its X is the number exiled this way.
fn exile_creature_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(you)).to_vec();
    let chosen = choose::select(g, you, item.object, Purpose::Exile, graveyard, select::creature_cards(select::ANY_NUMBER))?;
    let exiled = g.exile_all(&chosen);
    if !exiled.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(this, 0), Target::None, count_u32(exiled.len()));
    }
    Ok(())
}

/// "Each other creature gets -X/-X until end of turn, where X is the number of cards exiled this way" (CR 611.2c): every
/// creature but this one, whoever controls it, on the battlefield as the ability resolves.
fn shrink_the_others(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x as i32;
    effects::pump_creatures_other_than(g, item.object, &[item.object], -x, -x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_creature_cards,
        "When this creature enters, you may exile one or more creature cards from your graveyard. When you do, each other creature gets -X/-X until end of turn, where X is the number of cards exiled this way.",
    )],
    delayed: &[delayed_trigger(
        shrink_the_others,
        "When you do, each other creature gets -X/-X until end of turn, where X is the number of cards exiled this way.",
    )],
    ..oracle::SPECTER_OF_MORTALITY
};
