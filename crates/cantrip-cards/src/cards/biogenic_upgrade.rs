use cantrip_api::*;
use cantrip_mechanics::{counters_divided, double, targets};
use cantrip_oracle as oracle;

/// "Distribute three +1/+1 counters among one, two, or three target creatures, then double the number of +1/+1 counters on each
/// of those creatures." The targets and their shares were fixed as the spell went on the stack (CR 601.2d); one that has since
/// become illegal is left out and its counters are lost (CR 608.2b). "Those creatures" are the targets still legal, each one's
/// count doubled (CR 701.10e) after the counters are put on.
fn distribute_then_double(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    counters_divided(g, item, CounterKind::PlusOne);
    double::counters(g, &creatures, CounterKind::PlusOne);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(1, 3)], distribute_then_double),
    divide: Some(Divide { total: 3, reads: None }),
    ..oracle::BIOGENIC_UPGRADE
};
