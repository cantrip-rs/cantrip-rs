use cantrip_api::*;
use cantrip_mechanics::{effects, library, select, triggers};
use cantrip_oracle as oracle;

pub static BACK: CardDef = CardDef {
    ..oracle::SEASONED_CATHAR
};

/// "When this creature enters, you may search your library for a basic Plains card, reveal it, put it into your hand, then shuffle." (CR 608.2, 400.7, 701.24a).
fn search_library_basic_plains_card(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_8: Vec<ObjectId> = Vec::new();
    let mut hoisted_7: Vec<ObjectId> = Vec::new();
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    if g.may_search(item.controller, item.controller)
        && effects::ask_may(g, item.controller, item.object, Purpose::Optional)?
    {
        let shown_1 = library::searched_cards(g, item.controller);
        let can_1: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown_1
                .iter()
                .copied()
                .filter(|&id| {
                    q.chars(id).supertypes.basic
                        && q
                            .chars(id)
                            .has_subtype(q.words(item.object).subtype(Subtype::Plains))
                })
                .collect()
        };
        let found_1 = library::search(
            g,
            item.controller,
            item.controller,
            item.object,
            SearchDestination::Hand,
            select::Selection::listed(select::Count::UpTo(1), &shown_1, can_1),
        )?;
        let objects_2 = found_1.clone();
        let mut owners_2: Vec<PlayerId> = objects_2
            .iter()
            .map(|o| g.state().obj(*o).owner)
            .collect();
        owners_2.dedup();
        for owner in owners_2 {
            let cards: Vec<ObjectId> = objects_2
                .iter()
                .copied()
                .filter(|o| g.state().obj(*o).owner == owner)
                .collect();
            g.reveal(owner, &cards);
        }
        let objects_3 = found_1.clone();
        let moved_3 = g.return_to_hand_revealed(&objects_3);
        g.shuffle(item.controller);
        g.end_sentence();
        hoisted_5.extend(moved_3.clone());
        hoisted_6.extend(found_1.clone());
        hoisted_7.extend(objects_2.clone());
        hoisted_8
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

/// "Coven — {1}{W}{W}: Transform this creature. Activate only if you control three or more creatures with different powers." (CR 608.2).
fn transform_creature_activate_only_if(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(object) = ({
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    })
        .first()
        .copied() else { return Ok(()) };
    g.transform_by(object, item)?;
    Ok(())
}

/// "Coven — {1}{W}{W}: Transform this creature. Activate only if you control three or more creatures with different powers." (CR 608.2).
fn transform_creature_activate_only_if_if(q: &dyn Query, asker: &Asker) -> bool {
    ({
        let mut seen = Vec::new();
        for id in q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && asker.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
        {
            let value = q.chars(id).power;
            if !seen.contains(&value) {
                seen.push(value);
            }
        }
        seen.len() as i64
    }) >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            search_library_basic_plains_card,
            "When this creature enters, you may search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.",
        ),
        ability(
                cost("{1}{W}{W}"),
                transform_creature_activate_only_if,
                "Coven — {1}{W}{W}: Transform this creature. Activate only if you control three or more creatures with different powers.",
            )
            .condition(transform_creature_activate_only_if_if),
    ],
    back: Some(&BACK),
    ..oracle::AMBITIOUS_FARMHAND
};
