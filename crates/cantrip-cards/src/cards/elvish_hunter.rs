use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{1}{G}, {T}: Target creature doesn't untap during its controller's next untap step." (CR 608.2, 611.2a).
fn target_creature_doesnt_untap_during(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kinds_0 = {
        let mut all = Vec::new();
        if let Some(object) = objects_0 {
            all.push((object, EffectKind::DoesntUntap));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_0 {
                g.add_effect(
                    item.object,
                    object,
                    kind,
                    Duration::DuringItsControllersNextUntapStep,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{G}").and(TAP),
                target_creature_doesnt_untap_during,
                "{1}{G}, {T}: Target creature doesn't untap during its controller's next untap step.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::ELVISH_HUNTER
};
