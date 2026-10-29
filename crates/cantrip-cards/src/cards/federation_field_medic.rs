use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Draw a card if you control a permanent with a counter on it." The condition is checked as this ability resolves
/// (CR 608.2h); only permanents on the battlefield that you control count.
fn draw_if_you_control_a_permanent_with_a_counter(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let controls_a_permanent_with_a_counter = g.state().battlefield().iter().any(|&permanent| {
        g.controller(permanent) == item.controller && g.obj(permanent).has_counters()
    });
    if controls_a_permanent_with_a_counter {
        g.draw(item.controller, 1);
    }
    Ok(())
}

/// "When this creature enters, choose one" (CR 603.2, 603.3c, 700.2): the controller chooses a mode as the trigger is
/// put on the stack. The counter mode's target is chosen then too (CR 603.3d).
static MODES: ModalDef = modes(&[
    mode(
        effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn::<
            1,
            { Keywords::of(&[Keyword::Vigilance]).0 },
        >,
        "Put a +1/+1 counter on target creature. It gains vigilance until end of turn.",
    )
    .targets(&[targets::CREATURE]),
    mode(
        draw_if_you_control_a_permanent_with_a_counter,
        "Draw a card if you control a permanent with a counter on it.",
    ),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS,
        &MODES,
        "When this creature enters, choose one — Put a +1/+1 counter on target creature. It gains vigilance until end of turn; or draw a card if you control a permanent with a counter on it.",
    )],
    ..oracle::FEDERATION_FIELD_MEDIC
};
