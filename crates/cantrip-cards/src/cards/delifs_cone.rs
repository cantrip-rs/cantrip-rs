use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power." (CR 608.2, 611.2a, 119.3).
fn watch_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        g.end_sentence();
        return Ok(());
    }
    let amount = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .map_or(0, |o: ObjectId| i64::from(q.chars(o).power.unwrap_or(0)))
            .max(0) as i32
    };
    let before = g.history().life_gained(item.controller);
    g.gain_life(item.controller, amount);
    g.end_sentence();
    // "If you do": life was actually gained (CR 608.2c, 119.10), which a prevented or zero gain is not.
    if g.history().life_gained(item.controller) > before {
        let objects = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
        instruction(g, |g| {
            for &object in &objects {
                g.add_effect(item.object, object, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
            }
        });
        g.end_sentence();
    }
    Ok(())
}

/// "{T}, Sacrifice this artifact: This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn." (CR 608.2).
fn when_target_creature_control_attacks(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let Some(watched) = t_0.get(0).copied().and_then(|t: Target| t.object()) else { return Ok(()) };
    delayed::this_turn_watching(g, item, watched);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                SACRIFICE_THIS.and(TAP),
                when_target_creature_control_attacks,
                "{T}, Sacrifice this artifact: This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn.",
            )
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
            watch_2,
            "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. If you do, it assigns no combat damage this turn.",
        ),
    ],
    ..oracle::DELIFS_CONE
};
