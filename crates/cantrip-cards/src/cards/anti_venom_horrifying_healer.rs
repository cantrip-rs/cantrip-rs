use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If damage would be dealt to Anti-Venom" (CR 615.1): damage of any source, combat or not, dealt to it.
fn dealt_to_it(_: &dyn Query, asker: &Asker, d: &DamageEvent) -> bool {
    d.target == Target::Object(asker.source)
}

/// "... prevent that damage and put that many +1/+1 counters on him" (CR 615.5, 122.1): as many counters as the damage was,
/// as the effects applied before it have left it (CR 616.1f). Damage that can't be prevented is still dealt, but the counters
/// are still put on him (CR 615.12).
fn put_that_many_counters(g: &mut dyn Game, asker: &Asker, before: &Proposal, _: &Proposal) {
    let Proposal::Damage(d) = before else { return };
    g.add_counters(asker.source, CounterKind::PlusOne, d.amount as i32);
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_IF_CAST,
        effects::return_first_target_to_battlefield,
        "When Anti-Venom enters, if he was cast, return target creature card from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    preventions: &[PreventionDef {
        hint: "prevents the damage, and puts that many +1/+1 counters on him",
        applies: dealt_to_it,
        amount: PreventionAmount::All,
        then: Some(put_that_many_counters),
    }],
    ..oracle::ANTI_VENOM_HORRIFYING_HEALER
};
