use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// "When that creature dies this turn, destroy this artifact." (CR 608.2, 701.8a).
fn watch_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.destroy(item.object);
    Ok(())
}

/// "{2}, {T}: Target creature gains islandwalk until end of turn. When that creature dies this turn, destroy this artifact." (CR 608.2, 611.2a).
fn target_creature_gains_islandwalk_when(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if let Some(walk) = Keyword::landwalk(g.words(item.object).subtype(Subtype::Island)) {
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[walk])), Duration::EndOfTurn);
    }
    g.end_sentence();
    delayed::this_turn_watching(g, item, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                target_creature_gains_islandwalk_when,
                "{2}, {T}: Target creature gains islandwalk until end of turn. When that creature dies this turn, destroy this artifact.",
            )
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[
        triggered(
            triggers::THIS_DIES,
            watch_3,
            "When that creature dies this turn, destroy this artifact.",
        ),
    ],
    ..oracle::SANDALS_OF_ABDALLAH
};
