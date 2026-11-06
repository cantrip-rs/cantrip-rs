use cantrip_api::*;
use cantrip_mechanics::{effects, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Put two +1/+1 counters on target creature. It gains trample until end of turn." Both stop if the target is no longer legal
/// (CR 608.2b).
fn counters_and_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 2);
        trample_until_end_of_turn(g, item.object, creature);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::damage_first_target::<6>, "Konstrari Charm deals 6 damage to target creature with flying.").targets(&[targets::CREATURE_WITH_FLYING]),
    mode(counters_and_trample, "Put two +1/+1 counters on target creature. It gains trample until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::add_colorless::<3>, "Add {C}{C}{C}."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::KONSTRARI_CHARM };
