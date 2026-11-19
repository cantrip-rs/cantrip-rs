use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// "Sacrifice any number of artifacts, enchantments, and/or tokens. Return that many creature cards from your graveyard to the
/// battlefield." (CR 701.21a, 608.2c, 400.7): "that many" is how many were actually sacrificed, and the creature cards are chosen after
/// they are gone, so a creature card sacrificed as an artifact is in the graveyard to be chosen. With fewer creature cards than that,
/// all of them return; they enter together under the caster's control.
fn conquest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let sacrificed = effects::sacrifice_any_number_of_artifacts_enchantments_and_tokens(g, you, item.object)?;
    if sacrificed == 0 {
        return Ok(());
    }
    let graveyard = g.state().zone(Zone::graveyard(you)).to_vec();
    let rule = select::creature_cards(select::as_many_as_can(sacrificed));
    let chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, graveyard, rule)?;
    let cards: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|card| (card, you)).collect();
    g.put_all_onto_battlefield(&cards)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], conquest), ..oracle::LICH_KNIGHTS_CONQUEST };
