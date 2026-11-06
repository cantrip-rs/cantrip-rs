use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{U}{U}, {T}: Target creature gains shroud until end of turn. Activate only during your upkeep." (CR 608.2, 611.2a).
fn target_creature_gains_shroud_activate(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, target, Keyword::Shroud);
    Ok(())
}

/// "{U}{U}, {T}: Target creature gains shroud until end of turn. Activate only during your upkeep." (CR 602.5).
fn target_creature_gains_shroud_activate_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.controller)
        .is_some_and(|player: PlayerId| {
            q.state().turn.active == player && q.state().turn.step() == Step::Upkeep
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{U}{U}").and(TAP),
                target_creature_gains_shroud_activate,
                "{U}{U}, {T}: Target creature gains shroud until end of turn. Activate only during your upkeep.",
            )
            .targets(&[targets::CREATURE])
            .condition(target_creature_gains_shroud_activate_if),
    ],
    ..oracle::SVYELUNITE_PRIEST
};
