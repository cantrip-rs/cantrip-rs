use cantrip_api::*;
use cantrip_mechanics::library::{search_to_hand, Find};
use cantrip_mechanics::{solved, triggers};
use cantrip_oracle as oracle;

/// The 2/1 black Skeleton creature token this Case makes.
pub static SKELETON: CardDef = CardDef {
    name: "Skeleton Token",
    token_name: Some(&[Subtype::Skeleton]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Skeleton]),
    power: Some(2),
    toughness: Some(1),
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&SKELETON];

/// "When this Case enters, create a 2/1 black Skeleton creature token and suspect it" (CR 701.60a): the token made, and
/// only it, becomes suspected.
fn skeleton(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let token = g.card_id(&SKELETON);
    for made in g.create_tokens(token, item.controller, 1, false)? {
        g.suspect(made);
    }
    Ok(())
}

/// "To solve — You control no suspected Skeletons" (CR 719.3a), an intervening "if" asked as the end step begins and as the
/// ability resolves. "Skeletons" is a word of the Case's text (CR 612).
fn no_suspected_skeletons(q: &dyn Query, asker: &Asker) -> bool {
    let skeleton = q.words(asker.source).subtype(Subtype::Skeleton);
    !q.permanents_of(asker.controller).into_iter().any(|id| q.is_suspected(id) && q.chars(id).has_subtype(skeleton))
}

/// "Search your library for a card, put it into your hand, then shuffle" (CR 701.23).
fn tutor(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_hand(g, item.controller, item.object, Find::ANY, |_, _| true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, skeleton, "When this Case enters, create a 2/1 black Skeleton creature token and suspect it. (It has menace and can't block.)"),
        solved::to_solve(no_suspected_skeletons, "To solve \u{2014} You control no suspected Skeletons. (If unsolved, solve at the beginning of your end step.)"),
        solved::ability(cost("{1}{B}").and(SACRIFICE_THIS), tutor, "Solved \u{2014} {1}{B}, Sacrifice this Case: Search your library for a card, put it into your hand, then shuffle. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    ..oracle::CASE_OF_THE_STASHED_SKELETON
};
