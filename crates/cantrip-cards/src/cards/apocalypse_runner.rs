use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control with power 2 or less gains lifelink until end of turn and can't be blocked this turn" (CR 611.2a,
/// 509.1b): both effects on the creature, from this Vehicle, as one instruction (CR 608.2c). Nothing if the target is no longer legal as the
/// ability resolves, its power now over 2 included (CR 608.2b).
fn gains_lifelink_and_cant_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
        g.add_effect(item.object, creature, EffectKind::CantBeBlocked, Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            gains_lifelink_and_cant_be_blocked,
            "{T}: Target creature you control with power 2 or less gains lifelink until end of turn and can't be blocked this turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL_WITH_POWER_2_OR_LESS]),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::APOCALYPSE_RUNNER
};
