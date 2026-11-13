use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks and isn't blocked, you may gain control of target land defending player controls for as long as you control this creature. If you do, this creature assigns no combat damage this turn." (CR 115.1).
fn gain_control_target_land_defending_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Land) && q.event_subject().object().and_then(| o :
        ObjectId | q.defending_player(o)) == Some(q.controller(id))))
    )
}

/// "Whenever this creature attacks and isn't blocked, you may gain control of target land defending player controls for as long as you control this creature. If you do, this creature assigns no combat damage this turn." (CR 608.2, 611.2a).
fn gain_control_target_land_defending(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        effects::gain_control_of_target_while_you_control_source(g, item)?;
        g.end_sentence();
        if g.controller(land) == item.controller {
            instruction(g, |g| {
                g.add_effect(item.object, item.object, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
            });
        }
    }

    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
                gain_control_target_land_defending,
                "Whenever this creature attacks and isn't blocked, you may gain control of target land defending player controls for as long as you control this creature. If you do, this creature assigns no combat damage this turn.",
            )
            .targets(&[target(gain_control_target_land_defending_target_0)]),
    ],
    ..oracle::ORCISH_SQUATTERS
};
