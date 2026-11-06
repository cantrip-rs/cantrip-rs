use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: Gain control of target legendary creature for as long as you control this creature and this creature remains tapped." (CR 608.2, 611.2a).
fn gain_control_target_legendary_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(
        g,
        |g| {
            g.add_effect(
                item.object,
                target,
                EffectKind::Control(item.controller),
                Duration::WhileSourceTappedAndControlled,
            );
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                gain_control_target_legendary_creature,
                "{T}: Gain control of target legendary creature for as long as you control this creature and this creature remains tapped.",
            )
            .targets(&[targets::LEGENDARY_CREATURE]),
    ],
    may_not_untap: true,
    ..oracle::WILLOW_SATYR
};
