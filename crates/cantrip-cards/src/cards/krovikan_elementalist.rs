use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "{U}{U}: Target creature you control gains flying until end of turn. Sacrifice it at the beginning of the next end step." (CR 608.2, 611.2a).
fn target_creature_control_gains_flying(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    effects::first_target_gains_flying(g, item)?;
    g.end_sentence();
    delayed::at_next_end_step(g, item, 0, Target::Object(object));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}{R}"),
                effects::target_creature_gets::<1, 0>,
                "{2}{R}: Target creature gets +1/+0 until end of turn.",
            )
            .targets(&[targets::CREATURE]),
        ability(
                cost("{U}{U}"),
                target_creature_control_gains_flying,
                "{U}{U}: Target creature you control gains flying until end of turn. Sacrifice it at the beginning of the next end step.",
            )
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[
        delayed_trigger(delayed::sacrifice_subject, "Sacrifice it at the beginning of the next end step."),
    ],
    ..oracle::KROVIKAN_ELEMENTALIST
};
