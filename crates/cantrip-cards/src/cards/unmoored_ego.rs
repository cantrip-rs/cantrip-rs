use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "Choose a card name. Search target opponent's graveyard, hand, and library for up to four cards with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way." (CR 115.1).
fn choose_card_name_search_target_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Player(p) if q.state().opponents(asker.controller).collect:: < Vec <
        PlayerId > > ().contains(& p)
    )
}

/// "Choose a card name. Search target opponent's graveyard, hand, and library for up to four cards with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way." (CR 608.2, 121.1, 406.3).
fn choose_card_name_search_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let Answer::Name(Some(chosen_0)) = g
        .ask(
            item.controller,
            RequestKind::ChooseName {
                source: item.object,
                looked_at: Vec::new(),
                suggestion: None,
                kind: NameKind::ANY,
            },
        )? else { return Ok(()) };
    g.end_sentence();
    let Some(owner_1) = t_0.get(0).copied().and_then(|t: Target| t.player()) else { return Ok(()) };
    let mut left_1: u32 = 4u32;
    let mut found_1: Vec<ObjectId> = Vec::new();
    {
        let shown = g.state().zone(Zone::graveyard(owner_1)).to_vec();
        let can: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown
                .iter()
                .copied()
                .filter(|&id| {
                    Some(chosen_0)
                        .is_some_and(|chosen| library::object_has_name(q, id, chosen))
                })
                .collect()
        };
        if !can.is_empty() {
            let taken = choose::select(
                g,
                item.controller,
                item.object,
                Purpose::Exile,
                shown.clone(),
                select::Selection::listed(select::Count::UpTo(left_1), &shown, can),
            )?;
            left_1 -= taken.len() as u32;
            found_1.extend(taken);
        }
    }
    {
        let shown = g.state().zone(Zone::hand(owner_1)).to_vec();
        let can: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown
                .iter()
                .copied()
                .filter(|&id| {
                    Some(chosen_0)
                        .is_some_and(|chosen| library::object_has_name(q, id, chosen))
                })
                .collect()
        };
        if !can.is_empty() {
            let taken = choose::select(
                g,
                item.controller,
                item.object,
                Purpose::Exile,
                shown.clone(),
                select::Selection::listed(select::Count::UpTo(left_1), &shown, can),
            )?;
            left_1 -= taken.len() as u32;
            found_1.extend(taken);
        }
    }
    {
        let shown = library::searched_cards(g, owner_1);
        let can: Vec<ObjectId> = {
            let q: &dyn Query = g;
            shown
                .iter()
                .copied()
                .filter(|&id| {
                    Some(chosen_0)
                        .is_some_and(|chosen| library::object_has_name(q, id, chosen))
                })
                .collect()
        };
        let taken = library::search(
            g,
            item.controller,
            owner_1,
            item.object,
            SearchDestination::Exile,
            select::Selection::listed(select::Count::UpTo(left_1), &shown, can),
        )?;
        found_1.extend(taken);
    }
    let objects_2 = found_1.clone();
    g.exile_all(&objects_2);
    for player in t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.player())
        .into_iter()
        .collect::<Vec<PlayerId>>()
    {
        g.shuffle(player);
    }
    g.end_sentence();
    let amount_4 = {
        let q: &dyn Query = g;
        (objects_2
            .iter()
            .copied()
            .filter(|&o| q.state().obj(o).became.is_some())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| q.state().obj(id).zone.kind() == ZoneKind::Hand)
            .collect::<Vec<ObjectId>>()
            .len() as i64) as u32
    };
    let players_4 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.player())
        .into_iter()
        .collect::<Vec<PlayerId>>();
    for player in players_4 {
        g.draw(player, amount_4);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(choose_card_name_search_target_target_0)],
        choose_card_name_search_target,
    ),
    ..oracle::UNMOORED_EGO
};
