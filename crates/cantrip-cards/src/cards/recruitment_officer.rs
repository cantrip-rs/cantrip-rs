use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "{3}{W}: Look at the top four cards of your library. You may reveal a creature card with mana value 3 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 608.2, 400.7, 701.20a).
fn look_top_four_cards_library(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::library(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .take(4)
            .collect::<Vec<ObjectId>>()
    };
    g.look_at(item.controller, &objects_1);
    let selected_2_candidates = {
        let q: &dyn Query = g;
        objects_1
            .clone()
            .into_iter()
            .filter(|&id| {
                q.state().obj(id).became.is_none()
                    && q.state().obj(id).zone.kind() == ZoneKind::Library
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature) && (i64::from(q.mana_value(id))) <= 3
            })
            .collect::<Vec<ObjectId>>()
    };
    let selected_2 = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::Reveal,
        selected_2_candidates,
        select::Selection::of(select::Count::UpTo(1), &[]),
    )?;
    let objects_3 = selected_2.clone();
    let mut owners_3: Vec<PlayerId> = objects_3
        .iter()
        .map(|o| g.state().obj(*o).owner)
        .collect();
    owners_3.dedup();
    for owner in owners_3 {
        let cards: Vec<ObjectId> = objects_3
            .iter()
            .copied()
            .filter(|o| g.state().obj(*o).owner == owner)
            .collect();
        g.reveal(owner, &cards);
    }
    let objects_4 = selected_2.clone();
    g.return_to_hand_revealed(&objects_4);
    let cards_5: Vec<ObjectId> = {
        let q: &dyn Query = g;
        ({
            let without: Vec<ObjectId> = {
                let mut all: Vec<ObjectId> = Vec::new();
                for id in objects_4
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
                {
                    if !all.contains(&id) {
                        all.push(id);
                    }
                }
                all
            };
            objects_1
                .clone()
                .into_iter()
                .filter(|id| !without.contains(id))
                .collect::<Vec<ObjectId>>()
        })
            .into_iter()
            .filter(|&id| {
                q.state().obj(id).became.is_none()
                    && q.state().obj(id).zone.kind() == ZoneKind::Library
            })
            .collect::<Vec<ObjectId>>()
    }
        .into_iter()
        .filter(|&o| {
            g.state().obj(o).became.is_none()
                && g.state().obj(o).zone == Zone::library(item.controller)
        })
        .collect();
    library::rest_to_bottom_in_random_order(g, item.controller, &cards_5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{3}{W}"),
            look_top_four_cards_library,
            "{3}{W}: Look at the top four cards of your library. You may reveal a creature card with mana value 3 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
    ],
    ..oracle::RECRUITMENT_OFFICER
};
