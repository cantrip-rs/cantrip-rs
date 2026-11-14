use cantrip_api::*;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, double, triggers};
use cantrip_oracle as oracle;

/// "Double Worf's power until end of turn" (CR 701.10a-b): +X/+0, where X is his power as the ability resolves. A Worf who
/// has left the battlefield is gone (CR 400.7), so nothing happens.
fn double_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    double::power(g, item.object, item.object);
    Ok(())
}

/// "An untapped artifact and/or creature you control" as the permanents a cost taps (CR 118.3, 602.2b): either kind counts and
/// nothing is tapped twice, so an artifact creature is a single choice. Worf himself may be one of the two: the cost has no
/// {T} of its own, so a creature that came under your control this turn can pay it (CR 302.6).
fn untapped_artifact_or_creature_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller
        && !q.obj(id).tapped
        && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature))
}

/// "Put a +1/+1 counter on Worf." Nothing if he has left the battlefield (CR 400.7).
fn add_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ATTACK,
            double_power,
            "Whenever you attack while creatures you control have total power 8 or greater, double Worf's power until end of turn.",
        )
        .only_if(your_creatures_have_total_power_at_least::<8>),
        ability(
            Cost::tap_untapped(2, untapped_artifact_or_creature_you_control),
            add_counter,
            "Tap two untapped artifacts and/or creatures you control: Put a +1/+1 counter on Worf. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::WORF_HONOR_BOUND_WARRIOR
};
