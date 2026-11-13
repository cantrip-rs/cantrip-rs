use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Gain control of target creature an opponent controls for as long as you control this Vehicle. That creature can't attack
/// or block for as long as you control this Vehicle." Both last while `YOU_CONTROL_SOURCE` holds (CR 611.2b): if the Vehicle
/// has already left, neither does anything.
fn possess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The target is read first: once it is under the ability's controller it is no longer a creature an opponent controls.
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_control_of_target_while_you_control_source(g, item)?;
    g.end_sentence();
    effects::cant_attack_or_block_while_you_control_source(g, item, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            possess,
            "When this Vehicle enters, gain control of target creature an opponent controls for as long as you control this Vehicle. That creature can't attack or block for as long as you control this Vehicle.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::POSSESSION_ENGINE
};
