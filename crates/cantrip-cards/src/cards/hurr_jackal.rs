use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: Target creature can't be regenerated this turn." (CR 608.2, 611.2a).
fn target_creature_cant_be_regenerated(
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
                EffectKind::CantBeRegenerated,
                Duration::EndOfTurn,
            );
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                target_creature_cant_be_regenerated,
                "{T}: Target creature can't be regenerated this turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::HURR_JACKAL
};
