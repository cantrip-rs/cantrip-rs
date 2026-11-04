use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, triggers};
use cantrip_oracle as oracle;

/// "You may return another permanent you control to its owner's hand" (CR 603.6a, 608.2d): chosen as the ability resolves, not
/// targeted, up to one of the controller's other permanents; none chosen is not returning one. Flash is in the characteristics.
fn may_return_another_permanent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object).collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::ReturnToHand, others, select::cards(select::up_to(1)))?;
    g.return_to_hand(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, may_return_another_permanent, "When Ambrosia Whiteheart enters, you may return another permanent you control to its owner's hand."),
        triggered(
            triggers::LANDFALL,
            effects::this_gets::<1, 0>,
            "Landfall \u{2014} Whenever a land you control enters, Ambrosia Whiteheart gets +1/+0 until end of turn.",
        ),
    ],
    ..oracle::AMBROSIA_WHITEHEART
};
