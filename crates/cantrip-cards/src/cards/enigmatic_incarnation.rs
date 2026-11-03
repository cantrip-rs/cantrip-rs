use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, you may sacrifice another enchantment. If you do, search your library for a creature card with mana value equal to 1 plus the sacrificed enchantment's mana value, put that card onto the battlefield, then shuffle." (CR 608.2, 701.21a, 701.24a).
fn sacrifice_another_enchantment_if_do(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_7: Vec<ObjectId> = Vec::new();
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let (paid_0, _, before_0, _): (bool, Vec<ObjectId>, Vec<ObjectId>, Vec<ObjectId>) = {
        let objects_1 = {
            let q: &dyn Query = g;
            q.state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    !(item.object == id) && q.chars(id).is(CardType::Enchantment)
                })
                .collect::<Vec<ObjectId>>()
        };
        let mut picks_1: Vec<(PlayerId, ObjectId)> = Vec::new();
        {
            let eligible: Vec<ObjectId> = objects_1
                .iter()
                .copied()
                .filter(|&o| {
                    g.controller(o) == item.controller
                        && g.can_sacrifice(item.controller, o)
                })
                .collect();
            let taken = choose::select(
                g,
                item.controller,
                item.object,
                Purpose::Sacrifice,
                eligible,
                select::Selection::of(select::Count::UpTo(1).clone(), &[]),
            )?;
            for object in taken {
                picks_1.push((item.controller, object));
            }
        }
        let sacrificed_1 = g.sacrifice_each(&picks_1);
        let paid_1 = !picks_1.is_empty();
        (
            paid_1,
            sacrificed_1.clone(),
            {
                let q: &dyn Query = g;
                objects_1
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
                    .clone()
            },
            Vec::new(),
        )
    };
    g.end_sentence();
    if (paid_0 as i64) != 0 {
        let shown_2 = library::searched_cards(g, item.controller);
        let can_2: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown_2
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).is(CardType::Creature)
                        && (i64::from(q.mana_value(id)))
                            == ((1
                                + before_0
                                    .clone()
                                    .get(0)
                                    .copied()
                                    .map_or(0, |o: ObjectId| i64::from(q.mana_value(o)))))
                })
                .collect()
        };
        let found_2 = library::search(
            g,
            item.controller,
            item.controller,
            item.object,
            SearchDestination::Battlefield,
            select::Selection::listed(select::Count::UpTo(1), &shown_2, can_2),
        )?;
        let objects_3 = found_2
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
        g.shuffle(item.controller);
        g.end_sentence();
        hoisted_5.extend(moved_3.clone());
        hoisted_6.extend(found_2.clone());
        hoisted_7
            .extend({
                let q: &dyn Query = g;
                objects_3
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            sacrifice_another_enchantment_if_do,
            "At the beginning of your end step, you may sacrifice another enchantment. If you do, search your library for a creature card with mana value equal to 1 plus the sacrificed enchantment's mana value, put that card onto the battlefield, then shuffle.",
        ),
    ],
    ..oracle::ENIGMATIC_INCARNATION
};
