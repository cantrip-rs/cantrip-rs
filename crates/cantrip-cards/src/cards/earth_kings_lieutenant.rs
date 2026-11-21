use cantrip_api::*;
use cantrip_mechanics::effects::{counters_on_each, plus_one_counter_on_this};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each other Ally creature you control" (CR 122.1, 205.3m): the creatures with the Ally subtype, read
/// through the words of this card's text (CR 612.2), that its controller controls as it resolves, all but the Lieutenant, together.
fn counter_on_each_other_ally_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ally = g.words(item.object).subtype(Subtype::Ally);
    let allies: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature) && g.chars(id).has_subtype(ally))
        .collect();
    counters_on_each(g, &allies, CounterKind::PlusOne, 1);
    Ok(())
}

/// Trample is a core keyword read from its characteristics. "Whenever another Ally you control enters" is the shared trigger.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            counter_on_each_other_ally_creature,
            "When this creature enters, put a +1/+1 counter on each other Ally creature you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Ally) }>),
            plus_one_counter_on_this,
            "Whenever another Ally you control enters, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::EARTH_KINGS_LIEUTENANT
};
