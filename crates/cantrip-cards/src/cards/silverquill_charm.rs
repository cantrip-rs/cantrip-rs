use cantrip_api::*;
use cantrip_mechanics::{effects, exile_target, targets};
use cantrip_oracle as oracle;

/// Mode 2: "Exile target creature with power 2 or less" (CR 406.2): the target's power is judged as the spell is cast and again as it
/// resolves (CR 608.2b); one that no longer qualifies is not exiled.
fn exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::two_plus_one_counters_on_target, "Put two +1/+1 counters on target creature.").targets(&[targets::CREATURE]),
    mode(exile, "Exile target creature with power 2 or less.").targets(&[targets::CREATURE_WITH_POWER_2_OR_LESS]),
    mode(effects::each_opponent_loses_life_you_gain_life::<3>, "Each opponent loses 3 life and you gain 3 life."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SILVERQUILL_CHARM };
