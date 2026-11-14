use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Whenever you manifest dread" (CR 701.62b, 603.2): its controller manifested dread, once the process was complete,
/// even if none of it could be done. The cards that process put into their graveyard are carried along (`u32::MAX` for none),
/// for "put a card you put into your graveyard this way".
fn you_manifest_dread(_: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ManifestedDread { player, graveyard } = *event else { return None };
    let id = |card: Option<ObjectId>| card.map_or(u32::MAX, |c| c.0);
    (player == me.controller).then_some(Fired { subject: Target::None, data: [id(graveyard[0]), id(graveyard[1])] })
}

/// "Put a card you put into your graveyard this way into your hand": one of the cards the manifest dread put into the
/// graveyard that is still there (CR 400.7: a card that has left it is a new object and was not put there this way), put into
/// its owner's hand by the controller choosing it. With none left, nothing happens (the ruling).
fn put_one_into_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<ObjectId> = item
        .data
        .iter()
        .filter(|&&id| id != u32::MAX)
        .map(|&id| ObjectId(id))
        .filter(|&card| g.obj(card).zone == Zone::graveyard(you))
        .collect();
    let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, cards, select::cards(select::exactly(1)))?;
    g.return_to_hand(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::MANIFESTED_DREAD, you_manifest_dread),
        put_one_into_hand,
        "Whenever you manifest dread, put a card you put into your graveyard this way into your hand.",
    )],
    ..oracle::PARANORMAL_ANALYST
};
