use cantrip_api::*;
use cantrip_mechanics::{effects, lands, permanents_with_subtype_you_control, triggers};
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on each creature you control, where X is the number of Shrines you control" (CR 122.1, 608.2h, 612): X is
/// worked out as the ability resolves, the Shrines read through the words of this card's text, the Temple among them while it is
/// still on the battlefield; the counters go on every creature at once.
fn counters_for_each_shrine(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let shrine = g.words(item.object).subtype(Subtype::Shrine);
    let x = permanents_with_subtype_you_control(g, item.controller, shrine) as i16;
    effects::counters_on_creatures_you_control(g, item.controller, CounterKind::PlusOne, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            counters_for_each_shrine,
            "When Southern Air Temple enters, put X +1/+1 counters on each creature you control, where X is the number of Shrines you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Shrine) }>),
            effects::plus_one_counter_on_each_creature_you_control,
            "Whenever another Shrine you control enters, put a +1/+1 counter on each creature you control.",
        ),
    ],
    ..oracle::SOUTHERN_AIR_TEMPLE
};
