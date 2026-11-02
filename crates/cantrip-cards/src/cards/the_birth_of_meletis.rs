use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle." (CR 608.2, 400.7, 701.24a).
fn i_search_library_basic_plains(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let shown_0 = library::searched_cards(g, item.controller);
    let can_0: Vec<ObjectId> = {
        let q: &dyn Query = g;
        shown_0
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
    let found_0 = library::search(
        g,
        item.controller,
        item.controller,
        item.object,
        SearchDestination::Hand,
        select::Selection::listed(select::Count::UpTo(1), &shown_0, can_0),
    )?;
    let objects_1 = found_0.clone();
    let mut owners_1: Vec<PlayerId> = objects_1
        .iter()
        .map(|o| g.state().obj(*o).owner)
        .collect();
    owners_1.dedup();
    for owner in owners_1 {
        let cards: Vec<ObjectId> = objects_1
            .iter()
            .copied()
            .filter(|o| g.state().obj(*o).owner == owner)
            .collect();
        g.reveal(owner, &cards);
    }
    let objects_2 = found_0.clone();
    g.return_to_hand_revealed(&objects_2);
    g.shuffle(item.controller);
    Ok(())
}

pub static WALL: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(0),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Defender]),
    ..token!(Wall)
};

/// "II — Create a 0/4 colorless Wall artifact creature token with defender." (CR 608.2, 111.1).
fn ii_create_0_4_colorless(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_4 = Vec::new();
    let def_4_0 = g.card_id(&WALL);
    tokens_4.extend(std::iter::repeat((def_4_0, item.controller)).take(1));
    let words_4 = g.words(item.object);
    g.create_tokens_worded(&tokens_4, false, None, words_4)?;
    Ok(())
}

/// "III — You gain 2 life." (CR 608.2, 119.3).
fn iii_gain_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            i_search_library_basic_plains,
            "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.",
        ),
        chapter(
            2,
            ii_create_0_4_colorless,
            "II — Create a 0/4 colorless Wall artifact creature token with defender.",
        ),
        chapter(3, iii_gain_2_life, "III — You gain 2 life."),
    ],
    ..oracle::THE_BIRTH_OF_MELETIS
};

pub static EXTRA: &[&CardDef] = &[&WALL];
