use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Choose another target creature you control. For as long as Lwaxana Troi remains tapped, ..." (CR 611.2b): records the
/// chosen creature for as long as this creature remains tapped; nothing if the target is no longer legal (CR 608.2b).
fn choose_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::TriggersAgainWhileSourceTapped, Duration::WhileSourceTapped);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "You may choose not to untap Lwaxana Troi during your untap step." (CR 502.3)
    may_not_untap: true,
    abilities: &[ability(cost("{2}, {T}"), choose_creature, "{2}, {T}: Choose another target creature you control. For as long as Lwaxana Troi remains tapped, if a triggered ability of that creature triggers, that ability triggers an additional time.")
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::LWAXANA_TROI_CHARMING_GUEST
};
