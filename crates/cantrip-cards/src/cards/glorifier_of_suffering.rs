use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature or artifact. When you do, ..." (CR 701.21a, 603.12): the permanent is chosen as the
/// ability resolves, and the reflexive ability, with its up to two targets, exists only if one was sacrificed.
fn sacrifice_for_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_sacrifice_when_you_do(g, item, select::another_creature_or_artifact_permanents(select::up_to(1)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        sacrifice_for_counters,
        "When this creature enters, you may sacrifice another creature or artifact. When you do, put a +1/+1 counter on each of up to two target creatures.",
    )],
    delayed: &[delayed_trigger(effects::plus_one_counter_on_each_target, "When you do, put a +1/+1 counter on each of up to two target creatures.")
        .targets(&[targets::CREATURE.between(0, 2)])],
    ..oracle::GLORIFIER_OF_SUFFERING
};
