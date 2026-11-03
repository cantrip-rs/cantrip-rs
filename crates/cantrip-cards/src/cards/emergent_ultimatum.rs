use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "Search your library for up to three monocolored cards with different names and exile them. An opponent chooses one of those cards. Shuffle that card into your library. You may cast the other cards without paying their mana costs. Exile Emergent Ultimatum." (CR 608.2).
fn search_0_quality(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| (q.chars(id).colors.iter().count() as i64) == 1)
}

/// "Search your library for up to three monocolored cards with different names and exile them. An opponent chooses one of those cards. Shuffle that card into your library. You may cast the other cards without paying their mana costs. Exile Emergent Ultimatum." (CR 608.2, 406.3, 701.24a).
fn search_library_up_three_monocolored(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let found_0 = library::search(
        g,
        item.controller,
        item.controller,
        item.object,
        SearchDestination::Exile,
        select::Selection::new(select::Count::UpTo(3))
            .with(
                cantrip_api::select::part(
                    &(search_0_quality as SelectFn),
                    0,
                    "monocolored card",
                ),
            )
            .with(select::DIFFERENT_NAMES),
    )?;
    let objects_1 = found_0.clone();
    let moved_1 = g.exile_all(&objects_1);
    let among: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().opponents(item.controller).collect::<Vec<PlayerId>>()
    };
    let picked_0 = if among.len() > 1 {
        choose::player(
            g,
            item.controller,
            item.object,
            among,
            "Choose a player to make the choice",
            0,
        )?
    } else {
        among.first().copied()
    };
    let mut carried_4_0 = Default::default();
    picked_0
        .map(|present_4_0| -> Result<(), Suspend> {
            let selected_3_player = present_4_0;
            let selected_3_candidates = moved_1.clone();
            let selected_3 = choose::select(
                g,
                selected_3_player,
                item.object,
                Purpose::Choose,
                selected_3_candidates,
                select::Selection::of(select::Count::AsManyAsCan(1), &[]),
            )?;
            (carried_4_0,) = (selected_3.clone(),);
            Ok(())
        })
        .transpose()?;
    let objects_6 = carried_4_0.clone().into_iter().rev().collect::<Vec<ObjectId>>();
    g.put_into_library(&objects_6, LibraryPlace::Top);
    g.shuffle(item.controller);
    g.end_sentence();
    choose::cast_any_number_free(g, item.controller, item.object, &moved_1.clone())?;
    g.end_sentence();
    let objects_8 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    g.exile_all(&objects_8);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_library_up_three_monocolored),
    ..oracle::EMERGENT_ULTIMATUM
};
