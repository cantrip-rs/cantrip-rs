use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{3}, {T}: Target creature gets +2/+0 for as long as this artifact remains tapped." (CR 608.2, 611.2a).
fn target_creature_gets_2_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, EffectKind::Pump(2, 0), Duration::WhileSourceTapped);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}").and(TAP),
                target_creature_gets_2_0,
                "{3}, {T}: Target creature gets +2/+0 for as long as this artifact remains tapped.",
            )
            .targets(&[targets::CREATURE]),
    ],
    may_not_untap: true,
    ..oracle::ZELYON_SWORD
};
