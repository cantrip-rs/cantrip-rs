use cantrip_api::*;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_YOU_CONTROL_ENTERS, THIS_ENTERS};
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// "If this creature is suspected" (CR 701.60b, 603.4): an intervening "if", asked as the creatures enter and again as the
/// ability resolves.
fn this_is_suspected(q: &dyn Query, asker: &Asker) -> bool {
    q.is_suspected(asker.source)
}

/// "You may suspect one of the other creatures. If you do, this creature is no longer suspected" (CR 701.60a, 603.2c, 608.2c): the
/// player who controls this creature chooses up to one of the creatures that entered together, as the ability resolves. Only one
/// still on the battlefield can be suspected (CR 701.60b), and one already suspected can't become suspected again (CR 701.60d), so
/// choosing it would not be suspecting it: only the others are offered, and with none of them there is nothing to choose.
fn suspect_one_of_them_instead(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let them: Vec<ObjectId> = g.event_objects(item).iter().copied().filter(|&id| g.is_on_battlefield(id) && !g.is_suspected(id)).collect();
    let Some(&chosen) = choose::select(g, item.controller, item.object, Purpose::Choose, them, select::cards(select::up_to(1)))?.first() else {
        return Ok(());
    };
    g.suspect(chosen);
    g.unsuspect(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::suspect_this, "When this creature enters, suspect it. (It has menace and can't block.)"),
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            suspect_one_of_them_instead,
            "Whenever one or more other creatures you control enter, if this creature is suspected, you may suspect one of the other creatures. If you do, this creature is no longer suspected.",
        )
        .once_per_batch()
        .condition(this_is_suspected),
    ],
    ..oracle::FRANTIC_SCAPEGOAT
};
