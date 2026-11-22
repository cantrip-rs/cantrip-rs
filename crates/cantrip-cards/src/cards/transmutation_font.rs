use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, tokens};
use cantrip_oracle as oracle;

const BLOOD: u8 = 0;
const CLUE: u8 = 1;
const FOOD: u8 = 2;

/// "Create your choice of a Blood token, a Clue token, or a Food token." The choice is made as the ability resolves (CR 608.2d).
fn create_your_choice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = g.card_id(&CARD);
    let labels = (BLOOD..=FOOD).map(|option| Label::Card(card, option)).collect();
    match choose::one_of(g, item.controller, item.object, labels)? {
        BLOOD => tokens::create_blood(g, item.controller),
        CLUE => tokens::investigate(g, item.controller),
        _ => tokens::create_food(g, item.controller),
    }
}

/// "Search your library for an artifact card, put it onto the battlefield, then shuffle." (CR 701.23): the library is shuffled
/// whether or not a card is found.
fn search_for_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, select::artifact_cards(select::up_to(1)), false)?;
    Ok(())
}

/// An artifact token, as the cost's filter: the three sacrificed must also have different names (CR 201.2).
fn artifact_token(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.obj(id).token && q.chars(id).is(CardType::Artifact)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, create_your_choice, "{T}: Create your choice of a Blood token, a Clue token, or a Food token."),
        ability(
            cost("{3}, {T}").and(sacrifice_together(3, artifact_token, &select::DIFFERENT_NAMES)),
            search_for_an_artifact,
            "{3}, {T}, Sacrifice three artifact tokens with different names: Search your library for an artifact card, put it onto the battlefield, then shuffle. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    labels: &["Blood", "Clue", "Food"],
    ..oracle::TRANSMUTATION_FONT
};

pub static EXTRA: &[&CardDef] = &[&tokens::BLOOD, &tokens::CLUE, &tokens::FOOD];
