use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped." (CR 608.2, 611.2a).
fn gain_control_target_creature_as(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.gain_control(item.object, creature, item.controller, Duration::WhileSourceTappedAndControlled);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                gain_control_target_creature_as,
                "{T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped.",
            )
            .targets(&[targets::CREATURE]),
    ],
    may_not_untap: true,
    ..oracle::RUBINIA_SOULSINGER
};
