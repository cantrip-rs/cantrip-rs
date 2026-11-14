use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "This turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power." (CR 608.2, 611.2a, 119.3).
fn watch_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let paid_3 = effects::ask_may(g, item.controller, item.object, Purpose::Optional)?;
    if paid_3 {
        let amount_4 = {
            let q: &dyn Query = g;
            item
                .subject
                .object()
                .map_or(0, |o: ObjectId| i64::from(q.chars(o).power.unwrap_or(0)))
                .max(0) as i32
        };
        g.gain_life(item.controller, amount_4);
        g.end_sentence();
    }
    g.end_sentence();
    if (paid_3 as i64) != 0 {
        let objects_5 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
        instruction(
            g,
            |g| {
                for &object in &objects_5 {
                    g.add_effect(
                        item.object,
                        object,
                        EffectKind::NoCombatDamageAssignment,
                        Duration::EndOfTurn,
                    );
                }
            },
        );
        g.end_sentence();
        hoisted_6.extend(objects_5.clone());
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
