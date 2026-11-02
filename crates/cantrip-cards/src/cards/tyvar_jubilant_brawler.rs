use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "You may activate abilities of creatures you control as though those creatures had haste. +1: Untap up to one target creature. −2: Mill three cards, then you may return a creature card with mana value 2 or less from your graveyard to the battlefield." (CR 604.1).
fn untap_up_one_target_creature_rule(
    q: &dyn Query,
    asker: &Asker,
    creature: ObjectId,
) -> bool {
    q.chars(creature).is(CardType::Creature)
        && asker.controller == q.controller(creature)
}

/// "+1: Untap up to one target creature." (CR 115.1).
fn untap_up_one_target_creature_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "+1: Untap up to one target creature." (CR 608.2, 701.26a, 502.3).
fn untap_up_one_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0_raw = announced(g, item, 0, false);
    let t_0 = announced(g, item, 0, true);
    let objects_0 = {
        let q: &dyn Query = g;
        t_0_raw
            .get(0)
            .copied()
            .filter(|t| t_0.contains(t))
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_0: Vec<ObjectId> = objects_0
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_0 {
                g.untap(object);
            }
        },
    );
    Ok(())
}

/// "−2: Mill three cards, then you may return a creature card with mana value 2 or less from your graveyard to the battlefield." (CR 608.2, 701.17a).
fn mill_three_cards_return_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    let mills_1: Vec<(PlayerId, u32)> = Some(item.controller)
        .into_iter()
        .map(|p| (p, 3u32))
        .collect();
    g.mill_together(&mills_1);
    g.end_sentence();
    {
        let selected_2_candidates = {
            let q: &dyn Query = g;
            Some(item.controller)
                .into_iter()
                .flat_map(|p: PlayerId| q.state().zone(Zone::graveyard(p)).to_vec())
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| {
                    q.chars(id).is(CardType::Creature)
                        && (i64::from(q.mana_value(id))) <= 2
                })
                .collect::<Vec<ObjectId>>()
        };
        let selected_2 = choose::select(
            g,
            item.controller,
            item.object,
            Purpose::PutOntoBattlefield,
            selected_2_candidates,
            select::Selection::of(select::Count::UpTo(1), &[]),
        )?;
        if !selected_2.is_empty() {
            let objects_3 = selected_2
                .clone()
                .into_iter()
                .filter(|&o: &ObjectId| !g.state().obj(o).token)
                .collect::<Vec<ObjectId>>();
            let moved_3 = {
                let rows_3: Vec<(ObjectId, PlayerId, bool)> = objects_3
                    .iter()
                    .map(|&o| (o, item.controller, false))
                    .collect();
                let came_3 = g.put_all_onto_battlefield_as(&rows_3)?;
                came_3
                    .into_iter()
                    .filter(|&o| g.state().obj(o).zone.is(ZoneKind::Battlefield))
                    .collect::<Vec<ObjectId>>()
            };
            g.end_sentence();
            hoisted_4.extend(moved_3.clone());
            hoisted_5.extend(selected_2.clone());
            hoisted_6
                .extend({
                    let q: &dyn Query = g;
                    objects_3
                        .iter()
                        .copied()
                        .filter(|&o| q.state().obj(o).became.is_some())
                        .collect::<Vec<ObjectId>>()
                });
        }
    }
    Ok(())
}

/// "+1: Untap up to one target creature." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                loyalty(1),
                untap_up_one_target_creature,
                "+1: Untap up to one target creature.",
            )
            .targets(&[target(untap_up_one_target_creature_target_0).between(0, 1)])
            .sorcery_speed(),
        ability(
                loyalty(-2),
                mill_three_cards_return_creature,
                "−2: Mill three cards, then you may return a creature card with mana value 2 or less from your graveyard to the battlefield.",
            )
            .sorcery_speed(),
    ],
    activate_as_though_haste: &[untap_up_one_target_creature_rule],
    ..oracle::TYVAR_JUBILANT_BRAWLER
};
