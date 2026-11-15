use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{card_types_in_graveyard, counters_divided, double, targets, you_have_delirium};
use cantrip_oracle as oracle;

/// "Distribute two +1/+1 counters among one or two target creatures. Then if there are six or more card types among cards in your
/// graveyard, double the number of +1/+1 counters on those creatures." The targets and their shares were fixed as the ability went
/// on the stack (CR 601.2d); one that has since become illegal is left out and its counters are lost (CR 608.2b). "Those
/// creatures" are the targets still legal, each one's count of counters doubled (CR 701.10e) with every count read first. The six
/// types are counted afresh after the counters are placed - nothing the counters do changes a graveyard, but the sentence is "then
/// if", asked at that point, not an intervening "if".
fn distribute_then_double(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    counters_divided(g, item, CounterKind::PlusOne);
    if card_types_in_graveyard(g, item.controller) >= 6 {
        double::counters(g, &creatures, CounterKind::PlusOne);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        distribute_then_double,
        "Delirium \u{2014} Whenever this creature enters or attacks, if there are four or more card types among cards in your graveyard, distribute two +1/+1 counters among one or two target creatures. Then if there are six or more card types among cards in your graveyard, double the number of +1/+1 counters on those creatures.",
    )
    .targets(&[targets::CREATURE.between(1, 2)])
    .divide(Divide { total: 2, reads: None })
    .condition(you_have_delirium)],
    ..oracle::OMNIVOROUS_FLYTRAP
};
