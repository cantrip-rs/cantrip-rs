use cantrip_api::*;
use cantrip_mechanics::{forage, offspring, triggers};
use cantrip_oracle as oracle;

/// "You may forage. If you do, put two +1/+1 counters on it" (CR 701.61a, 608.2d): the counters go on the creature only if the
/// forage was paid, by exiling three cards from your graveyard or sacrificing a Food.
fn may_forage_then_two_counters_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if forage::may_forage(g, item)? {
        g.add_counters(item.object, CounterKind::PlusOne, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(
            triggers::THIS_ENTERS,
            may_forage_then_two_counters_on_this,
            "When this creature enters, you may forage. If you do, put two +1/+1 counters on it.",
        ),
    ],
    ..oracle::BUSHY_BODYGUARD
};
