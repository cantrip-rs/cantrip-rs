use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, choose, lands, select, statics, targets, triggers::ATTACHED_CREATURE_DIES_ABOUT_IT};
use cantrip_oracle as oracle;

/// "Mill cards equal to its power. Return this card to its owner's hand and up to one creature card milled this way to the
/// battlefield under your control." The power is the creature's as it last existed on the battlefield (CR 608.2h), a power less
/// than 0 milling none; the cards milled are all of them, you looking at what went where, and the one chosen is put onto the
/// battlefield from where milling left it (a replacement effect may have made it exile, CR 701.17c). The Aura is the card it became
/// in its owner's graveyard (CR 400.7f), returned only if it is still there as the ability resolves; one that has left
/// it since (the 2011-09-22 Angelic Destiny ruling) stays where it went, but the creature card is still put onto the battlefield.
fn mill_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let power = g.chars(creature).power.unwrap_or(0).max(0) as u32;
    let milled = g.mill(item.controller, power);
    if let Some(aura) = g.found_aura_after_enchanted_left(item.object, creature) {
        g.return_to_hand(&[aura]);
    }
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield, milled, select::CREATURE_CARD)?;
    let rows: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|card| (card, item.controller)).collect();
    g.put_all_onto_battlefield(&rows)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    abilities: &[triggered(
        ATTACHED_CREATURE_DIES_ABOUT_IT,
        mill_and_return,
        "When enchanted creature dies, mill cards equal to its power. Return this card to its owner's hand and up to one creature card milled this way to the battlefield under your control.",
    )],
    statics: &[
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Avatar) }>, "Enchanted creature gets +1/+1 for each creature card in your graveyard and is an Avatar in addition to its other types."),
        static_ability(
            Layer::PtModify,
            attached_object,
            statics::plus_one_per_card_type_in_your_graveyard::<{ CardType::Creature as u8 }>,
            "Enchanted creature gets +1/+1 for each creature card in your graveyard and is an Avatar in addition to its other types.",
        )
        .continues(),
    ],
    ..oracle::AVATAR_DESTINY
};
