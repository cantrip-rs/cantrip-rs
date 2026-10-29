use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Return each other nonland permanent that entered this turn to its owner's hand." All at once, as a plain move (CR 400.7).
/// "Entered this turn" is asked of the event history (it is the object that move made), "nonland" as
/// each permanent is now.
fn return_the_newcomers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let newcomers: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != item.object && !g.chars(id).is(CardType::Land) && history::entered_this_turn(g.history(), id))
        .collect();
    g.return_to_hand(&newcomers);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, return_the_newcomers, "When Renet enters, return each other nonland permanent that entered this turn to its owner's hand.")],
    ..oracle::RENET_TEMPORAL_APPRENTICE
};
