use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_mechanics::{choose, effects, saddle, select};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while saddled, it gets +1/+2 until end of turn. Then you may return any number of creatures that
/// saddled it this turn to their owner's hand" (CR 702.171c, 608.2c, 608.2d): the creatures tapped to pay for any saddle ability of
/// this Possum this turn that are still on the battlefield as the ability resolves (a creature that saddled it twice is one creature,
/// and one that left the battlefield is a different object by now, CR 400.7), the player taking none, some or all of them. They go
/// to their owners' hands together.
fn grow_then_may_return_the_saddlers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gets::<1, 2>(g, item)?;
    g.end_sentence();
    let mut saddlers: Vec<ObjectId> = Vec::new();
    for creature in g.history().saddled(item.object).collect::<Vec<_>>() {
        if !g.is_on_battlefield(creature) {
            continue;
        }
        if !saddlers.contains(&creature) {
            saddlers.push(creature);
        }
    }
    let rule = Selection::new(select::ANY_NUMBER).called("creature");
    let chosen = choose::select(g, item.controller, item.object, Purpose::ReturnToHand, saddlers, rule)?;
    g.return_to_hand(&chosen);
    Ok(())
}

/// "Saddle 1" (CR 702.171a) is the shared mechanic; "attacks while saddled" is judged as the Possum attacks (CR 702.171b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            grow_then_may_return_the_saddlers,
            "Whenever this creature attacks while saddled, it gets +1/+2 until end of turn. Then you may return any number of creatures that saddled it this turn to their owner's hand.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::RAMBLING_POSSUM
};
