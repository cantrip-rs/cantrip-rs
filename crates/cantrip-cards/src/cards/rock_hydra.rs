use cantrip_api::*;
use cantrip_mechanics::{effects, prevention};
use cantrip_oracle as oracle;

/// The +1/+1 counters on this creature (CR 122.1).
fn plus_one_counters(q: &dyn Query, hydra: ObjectId) -> u32 {
    q.obj(hydra).counters(CounterKind::PlusOne) as u32
}

/// "For each 1 damage that would be dealt to this creature, if it has a +1/+1 counter on it" (CR 615.1): damage from any
/// source, combat or not, to it while it has one.
fn dealt_to_it_with_a_counter(q: &dyn Query, asker: &Asker, d: &DamageEvent) -> bool {
    d.target == Target::Object(asker.source) && plus_one_counters(q, asker.source) > 0
}

/// "... prevent that 1 damage": each 1 damage it has a +1/+1 counter for, so as much of the damage, as the effects applied
/// before it have left it (CR 616.1f), as it has counters.
fn one_for_each_counter(q: &dyn Query, asker: &Asker, d: &DamageEvent) -> u32 {
    d.amount.min(plus_one_counters(q, asker.source))
}

/// "... remove a +1/+1 counter from it": one for each 1 damage it has a counter for, as the effect applies to the damage
/// (`PreventionDef::then`), so another source's damage dealt at the same time meets only the counters left. Damage that
/// can't be prevented is dealt, but the counters are still removed (the ruling; CR 615.12).
fn remove_a_counter_for_each(g: &mut dyn Game, asker: &Asker, before: &Proposal, _: &Proposal) {
    let Proposal::Damage(d) = before else { return };
    let removed = one_for_each_counter(g, asker, d);
    g.remove_counters(asker.source, CounterKind::PlusOne, removed as i32);
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    preventions: &[PreventionDef {
        hint: "removes a +1/+1 counter for each 1 damage it prevents",
        applies: dealt_to_it_with_a_counter,
        amount: PreventionAmount::Relative(one_for_each_counter),
        then: Some(remove_a_counter_for_each),
    }],
    abilities: &[
        ability(cost("{R}"), prevention::next_1_damage_to_this_this_turn,
            "{R}: Prevent the next 1 damage that would be dealt to this creature this turn."),
        ability(cost("{R}{R}{R}"), effects::plus_one_counter_on_this,
            "{R}{R}{R}: Put a +1/+1 counter on this creature. Activate only during your upkeep.")
            .condition(cantrip_mechanics::your_upkeep),
    ],
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to this creature this turn.")],
    ..oracle::ROCK_HYDRA
};
