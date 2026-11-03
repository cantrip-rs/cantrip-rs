use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{2}, {T}: Target creature you control gets +2/-2 for as long as this artifact remains tapped." (CR 608.2, 611.2a).
fn target_creature_control_gets_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let duration_0 = Duration::WhileSourceTapped;
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, EffectKind::Pump(2, -2), duration_0);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                target_creature_control_gets_2,
                "{2}, {T}: Target creature you control gets +2/-2 for as long as this artifact remains tapped.",
            )
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    may_not_untap: true,
    ..oracle::ASHNODS_BATTLE_GEAR
};
