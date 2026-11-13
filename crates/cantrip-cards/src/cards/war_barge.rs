use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When this artifact leaves the battlefield this turn, destroy that creature." (CR 608.2, 701.8a).
fn watch_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_4 = g.event_objects(item).to_vec();
    g.destroy_all_no_regen(&objects_4);
    Ok(())
}

/// "{3}: Target creature gains islandwalk until end of turn. When this artifact leaves the battlefield this turn, destroy that creature. A creature destroyed this way can't be regenerated." (CR 608.2, 611.2a).
fn target_creature_gains_islandwalk_when(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if let Some(walk) = Keyword::landwalk(g.words(item.object).subtype(Subtype::Island)) {
        effects::gain_until_end_of_turn(g, item.object, creature, walk);
    }
    g.end_sentence();
    g.delayed_trigger_about(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        &[creature],
    );
    Ok(())
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
