use cantrip_api::*;
use cantrip_mechanics::{regenerate, targets};
use cantrip_oracle as oracle;

/// "{R}{R}, {T}: Target creature can't be regenerated this turn." (CR 608.2, 611.2a).
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

/// Regenerate the legal target (CR 701.19a).
fn regenerate_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    regenerate::target(g, item, 0);
    Ok(())
}

const BLACK_OR_GREEN_CREATURE: TargetSpec = targets::creature_of_either_color::<{ Color::Black as u8 }, { Color::Green as u8 }>();

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{R}{R}").and(TAP),
                target_creature_cant_be_regenerated,
                "{R}{R}, {T}: Target creature can't be regenerated this turn.",
            )
            .targets(&[targets::CREATURE]),
        ability(
                cost("{B}{B}{R}").and(TAP),
                regenerate_creature,
                "{B}{B}{R}, {T}: Regenerate target black or green creature.",
            )
            .targets(&[BLACK_OR_GREEN_CREATURE]),
        ability(
                cost("{R}{G}{G}").and(TAP),
                regenerate_creature,
                "{R}{G}{G}, {T}: Regenerate target black or green creature.",
            )
            .targets(&[BLACK_OR_GREEN_CREATURE]),
    ],
    ..oracle::ORCISH_HEALER
};
