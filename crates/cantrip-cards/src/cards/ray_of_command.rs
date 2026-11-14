use cantrip_api::*;
use cantrip_mechanics::{haste_until_end_of_turn, targets, triggers};
use cantrip_oracle as oracle;

/// "When you lose control of the creature, tap it." (CR 608.2, 701.26a).
fn watch_4(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_5 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    g.tap_together(&objects_5);
    Ok(())
}

/// "Untap target creature an opponent controls and gain control of it until end of turn. That creature gains haste until end of turn. When you lose control of the creature, tap it." (CR 608.2, 611.2a, 701.26b).
fn untap_target_creature_opponent_controls(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(watched) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.untap(watched);
        g.gain_control(item.object, watched, item.controller, Duration::EndOfTurn);
    });
    g.end_sentence();
    haste_until_end_of_turn(g, item.object, watched);
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::WhenEvent,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::Object(watched),
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_AN_OPPONENT_CONTROLS],
        untap_target_creature_opponent_controls,
    ),
    delayed: &[
        triggered(
            triggers::YOU_LOSE_CONTROL_OF_THIS,
            watch_4,
            "When you lose control of the creature, tap it.",
        ),
    ],
    ..oracle::RAY_OF_COMMAND
};
