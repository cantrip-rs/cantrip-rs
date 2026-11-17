use cantrip_api::*;
use cantrip_mechanics::{choose, library, select};
use cantrip_oracle as oracle;

/// "Search your library for up to three monocolored cards with different names and exile them. An opponent chooses one of those cards. Shuffle that card into your library. You may cast the other cards without paying their mana costs. Exile Emergent Ultimatum." (CR 608.2).
fn search_0_quality(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| q.chars(id).colors.is_monocolored())
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
    let exiled = g.exile_all(&found_0);
    let chosen = choose::opponent_chooses_one(g, item.controller, item.object, Purpose::Choose, exiled.clone())?;
    // "Shuffle that card into your library": the shuffle happens when no card was chosen too (CR 701.24d).
    match chosen {
        Some(card) => {
            g.put_into_library(&[card], LibraryPlace::ShuffledIn);
        }
        None => g.shuffle(item.controller),
    }
    g.end_sentence();
    let others: Vec<ObjectId> = exiled.into_iter().filter(|&card| Some(card) != chosen).collect();
    choose::cast_any_number_free(g, item.controller, item.object, &others)?;
    g.end_sentence();
    g.exile_all(&[item.object]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_library_up_three_monocolored),
    ..oracle::EMERGENT_ULTIMATUM
};
