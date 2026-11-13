use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: Target creature with flying has base power 0 until end of turn." (CR 608.2, 611.2a).
fn target_creature_flying_has_base(
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
                EffectKind::SetPower(0),
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
                target_creature_flying_has_base,
                "{T}: Target creature with flying has base power 0 until end of turn.",
            )
            .targets(&[targets::CREATURE_WITH_FLYING]),
    ],
    ..oracle::ISLAND_OF_WAK_WAK
};
