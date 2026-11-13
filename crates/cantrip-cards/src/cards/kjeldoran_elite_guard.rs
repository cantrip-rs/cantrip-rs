use cantrip_api::*;
use cantrip_mechanics::{combat, targets, triggers};
use cantrip_oracle as oracle;

/// "When that creature leaves the battlefield this turn, sacrifice this creature." (CR 608.2, 701.21a).
fn watch_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.sacrifice(item.controller, &[item.object]);
    Ok(())
}

/// "{T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat." (CR 608.2, 611.2a).
fn target_creature_gets_2_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 2),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    g.end_sentence();
    let Some(watched) = t_0.get(0).copied().and_then(|t: Target| t.object()) else { return Ok(()) };
    g.delayed_trigger(
        DelayedWhen::WatchingThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::Object(watched),
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                target_creature_gets_2_2,
                "{T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat.",
            )
            .targets(&[targets::CREATURE])
            .condition(combat::activate_only_during_combat),
    ],
    delayed: &[
        triggered(
            triggers::THIS_LEAVES,
            watch_3,
            "When that creature leaves the battlefield this turn, sacrifice this creature.",
        ),
    ],
    ..oracle::KJELDORAN_ELITE_GUARD
};
