use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Put any number of target artifact cards from target player's graveyard on top of their library in any order." (CR 115.1).
fn put_any_number_target_artifact_target_1(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && ! q.state().obj(id).card_copy && ((q.chars(id)
        .is(CardType::Artifact) && (q.state().obj(id).zone.kind() == ZoneKind::Graveyard
        && (q.chosen_targets().is_empty() || q.chosen_targets().get(0).copied()
        .and_then(| t : Target | t.player()) == Some(q.state().obj(id).owner)))))
    )
}

/// "Put any number of target artifact cards from target player's graveyard on top of their library in any order." (CR 608.2).
fn put_any_number_target_artifact(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    // The destination is the targeted player's library; an illegal player supplies no information (CR 608.2b).
    if !g.target_legal(item, 0) {
        return Ok(());
    }
    let t_1 = announced(g, item, 1, true);
    let objects_0 = t_1.iter().filter_map(|t| t.object()).collect::<Vec<ObjectId>>();
    {
        let owners_0: Vec<PlayerId> = g.state().apnap().collect();
        let mut order_0 = Vec::new();
        for owner_0 in owners_0 {
            let cards_0: Vec<ObjectId> = objects_0
                .iter()
                .copied()
                .filter(|&o| g.state().obj(o).owner == owner_0)
                .collect();
            order_0
                .extend(
                    library::ask_library_order(
                        g,
                        owner_0,
                        item.object,
                        &cards_0,
                        DestinationKind::LibraryTop,
                    )?,
                );
        }
        g.put_into_library(&order_0, LibraryPlace::Top)
    };
    Ok(())
}

/// "Put any number of target artifact cards from target player's graveyard on top of their library in any order." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::PLAYER,
            target(put_any_number_target_artifact_target_1)
                .between(0, TargetSpec::ANY_NUMBER),
        ],
        put_any_number_target_artifact,
    ),
    ..oracle::DRAFNAS_RESTORATION
};
