use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{2}, {T}: Target creature gets +0/+2 for as long as this artifact remains tapped." (CR 608.2, 611.2a).
fn target_creature_gets_0_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(
        g,
        |g| {
            g.add_effect(
                item.object,
                target,
                EffectKind::Pump(0, 2),
                Duration::WhileSourceTapped,
            );
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                target_creature_gets_0_2,
                "{2}, {T}: Target creature gets +0/+2 for as long as this artifact remains tapped.",
            )
            .targets(&[targets::CREATURE]),
    ],
    may_not_untap: true,
    ..oracle::SPIRIT_SHIELD
};
