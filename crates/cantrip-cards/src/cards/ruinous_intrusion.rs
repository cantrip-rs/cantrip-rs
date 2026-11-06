use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Exile target artifact or enchantment. Put X +1/+1 counters on target creature you control, where X is the mana value
/// of the permanent exiled this way: as it last existed on the battlefield (CR 608.2h), and zero when nothing was
/// exiled this way. The counters go on only while the creature target is still legal; the exile happens either way.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut x = 0;
    if let Some(permanent) = targets::object(g, item, 0) {
        let value = g.mana_value(permanent) as i32;
        let exiled = g.exile(permanent);
        if g.obj(exiled).zone.is(ZoneKind::Exile) {
            x = value;
        }
    }
    if x > 0
        && item.target_count > 1
        && let Some(creature) = targets::object(g, item, 1)
    {
        g.add_counters(creature, CounterKind::PlusOne,(x) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_ENCHANTMENT, targets::CREATURE_YOU_CONTROL], effect),
    ..oracle::RUINOUS_INTRUSION
};
