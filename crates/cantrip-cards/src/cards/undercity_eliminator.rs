use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, you may sacrifice an artifact or creature. When you do, exile target creature an opponent
/// controls." (CR 701.21a, 603.12): the permanent is chosen as the ability resolves, not targeted, and declining is the empty
/// answer to an up-to-one choice; this creature is itself a creature to sacrifice. The reflexive ability exists only for a
/// permanent actually sacrificed, whether or not this creature is still on the battlefield, and its target is chosen as it goes
/// on the stack.
fn sacrifice_to_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::artifact_or_creature_permanents(select::up_to(1));
    if effects::may_sacrifice(g, you, item.object, rule)?.is_empty() {
        return Ok(());
    }
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, you, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        sacrifice_to_exile,
        "When this creature enters, you may sacrifice an artifact or creature. When you do, exile target creature an opponent controls.",
    )],
    delayed: &[delayed_trigger(effects::exile_first_target, "When you do, exile target creature an opponent controls.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::UNDERCITY_ELIMINATOR
};
