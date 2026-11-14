use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "When this artifact leaves the battlefield this turn, destroy that creature." (CR 608.2, 701.8a).
fn watch_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_4 = {
        let q: &dyn Query = g;
        about(q, item, 0)
    };
    g.destroy_all_no_regen(&objects_4);
    Ok(())
}

/// "{3}: Target creature gains islandwalk until end of turn. When this artifact leaves the battlefield this turn, destroy that creature. A creature destroyed this way can't be regenerated." (CR 608.2, 611.2a).
fn target_creature_gains_islandwalk_when(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::AddKeywords(
            Keyword::landwalk(q.words(item.object).subtype(Subtype::Island))
                .map_or(Keywords::NONE, |walk| Keywords::of(&[walk])),
        )
    };
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    let about: Vec<ObjectId> = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.delayed_trigger_about(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &about,
    );
    Ok(())
}

/// "When this artifact leaves the battlefield this turn, destroy that creature." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    assert_eq!(part, 0, "one real-object list");
    q.event_objects(item).to_vec()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}"),
                target_creature_gains_islandwalk_when,
                "{3}: Target creature gains islandwalk until end of turn. When this artifact leaves the battlefield this turn, destroy that creature. A creature destroyed this way can't be regenerated.",
            )
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[
        triggered(
            triggers::THIS_LEAVES,
            watch_3,
            "When this artifact leaves the battlefield this turn, destroy that creature.",
        ),
    ],
    ..oracle::WAR_BARGE
};
