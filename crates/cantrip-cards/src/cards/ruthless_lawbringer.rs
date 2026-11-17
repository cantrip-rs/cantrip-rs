use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature. When you do, destroy target nonland permanent." (CR 701.21a, 603.12): the creature is
/// chosen as the triggered ability resolves and may be declined; the reflexive trigger is created only if one was actually
/// sacrificed, and chooses its target then.
fn sacrifice_to_destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::another_creature_permanents(select::up_to(1)))?.is_empty() {
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(g.card_id(&CARD), 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        sacrifice_to_destroy,
        "When this creature enters, you may sacrifice another creature. When you do, destroy target nonland permanent.",
    )],
    delayed: &[delayed_trigger(effects::destroy_first_target, "When you do, destroy target nonland permanent.").targets(&[targets::NONLAND_PERMANENT])],
    ..oracle::RUTHLESS_LAWBRINGER
};
