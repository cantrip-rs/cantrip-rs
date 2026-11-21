use cantrip_api::*;
use cantrip_mechanics::{effects, learn, triggers};
use cantrip_oracle as oracle;

/// "If there's a Lesson card in your graveyard" (CR 603.4, 205.3k): the intervening "if" of the dies trigger, asked as it would
/// trigger (Walltop Sentries is already in the graveyard, and is not a Lesson) and again as it resolves; "your" is its controller's
/// graveyard, so a Lesson in an opponent's does not count.
fn a_lesson_is_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    learn::lesson_cards_in_graveyard(q, asker.controller) > 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        effects::gain_life_amount::<2>,
        "When this creature dies, if there's a Lesson card in your graveyard, you gain 2 life.",
    )
    .condition(a_lesson_is_in_your_graveyard)],
    ..oracle::WALLTOP_SENTRIES
};
