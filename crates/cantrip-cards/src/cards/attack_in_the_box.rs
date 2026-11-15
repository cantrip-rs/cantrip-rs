use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "You may have it get +4/+0 until end of turn. If you do, sacrifice it at the beginning of the next end step." (CR 608.2d,
/// 611.2a, 603.7): the sacrifice is a delayed trigger about this object whether or not it is still there to pump (CR 400.7), and
/// it sacrifices only that same object (CR 603.7c).
fn pump_then_sacrifice_at_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        return Ok(());
    }
    effects::this_gets::<4, 0>(g, item)?;
    delayed::at_next_end_step(g, item, 0, Target::Object(item.object));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        pump_then_sacrifice_at_end_step,
        "Whenever this creature attacks, you may have it get +4/+0 until end of turn. If you do, sacrifice it at the beginning of the next end step.",
    )],
    delayed: &[delayed_trigger(delayed::sacrifice_subject, "Sacrifice it at the beginning of the next end step.")],
    ..oracle::ATTACK_IN_THE_BOX
};
