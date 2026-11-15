use cantrip_api::*;
use cantrip_mechanics::{crew, effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more land cards are put into your graveyard from your library" (CR 603.2c, 603.10): each land card that
/// moves from its owner's library to its owner's graveyard is an event about the card now in the graveyard, and
/// `.once_per_batch()` makes the lands that arrive together (one mill) a single trigger. The cards are those of the ability's
/// controller; a land put into an opponent's graveyard, or into your graveyard from anywhere else, does not trigger it.
fn lands_put_into_your_graveyard_from_your_library(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, from, to, .. }
            if *from == Zone::library(me.controller) && *to == Zone::graveyard(me.controller) && q.chars(*new).is(CardType::Land) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Put them onto the battlefield tapped" (CR 608.2c, 400.7): the land cards of the batch, together, under the ability's
/// controller. One that has left the graveyard since is a new object and is not put (the engine skips it).
fn put_them_onto_the_battlefield_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<(ObjectId, PlayerId, bool)> = g.event_objects(item).iter().map(|&land| (land, item.controller, true)).collect();
    g.put_all_onto_battlefield_as(&lands)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, |g, item| effects::may_mill(g, item, 2), "Whenever this Vehicle attacks, you may mill two cards."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, lands_put_into_your_graveyard_from_your_library),
            put_them_onto_the_battlefield_tapped,
            "Whenever one or more land cards are put into your graveyard from your library, put them onto the battlefield tapped.",
        )
        .once_per_batch(),
        crew::crew(1, "Crew 1"),
    ],
    ..oracle::HEDGE_SHREDDER
};
