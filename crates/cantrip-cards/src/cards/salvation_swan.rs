use cantrip_api::*;
use cantrip_mechanics::{delayed, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Return it to the battlefield under its owner's control with a flying counter on it" (CR 603.7, 122.6): the card exiled, or
/// every card of a merged permanent (CR 730.3c), if still in exile; it enters with the counter.
fn return_with_a_flying_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::return_exiled_with_counters(g, item, &[(CounterKind::Flying, 1)])
}

pub static CARD: CardDef = CardDef {
    // The shared slot-0 body exiles the target and sets up the delayed ability about the card exiled, only if it is in exile.
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Bird) }>),
        delayed::exile_target_then_return_at_next_end_step_0,
        "Whenever this creature or another Bird you control enters, exile up to one target creature you control without flying. Return it to the battlefield under its owner's control with a flying counter on it at the beginning of the next end step.",
    )
    .targets(&[up_to_one(targets::CREATURE_YOU_CONTROL_WITHOUT_FLYING.filter)])],
    delayed: &[delayed_trigger(
        return_with_a_flying_counter,
        "Return it to the battlefield under its owner's control with a flying counter on it at the beginning of the next end step.",
    )],
    ..oracle::SALVATION_SWAN
};
