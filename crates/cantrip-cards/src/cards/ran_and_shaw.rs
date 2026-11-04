use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST;
use cantrip_mechanics::{effects, firebending};
use cantrip_oracle as oracle;

/// "...if you cast them and there are three or more Dragon and/or Lesson cards in your graveyard" (CR 603.4): the second half of
/// this intervening "if" (the first is the trigger, `THIS_ENTERS_IF_CAST`), asked as it triggers and again as it resolves. Dragon and
/// Lesson are words of this card's text (CR 612); the cards are counted by the subtypes the layers give them in the
/// graveyard (CR 400.1, 205.3), so a Changeling (CR 702.73a) or a card Maskwood Nexus changed counts, a card with both counting once.
fn three_dragon_or_lesson_cards_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    let words = q.words(asker.source);
    let (dragon, lesson) = (words.subtype(Subtype::Dragon), words.subtype(Subtype::Lesson));
    let graveyard = q.state().zone(Zone::graveyard(asker.controller));
    graveyard.iter().filter(|&&card| q.chars(card).has_subtype(dragon) || q.chars(card).has_subtype(lesson)).count() >= 3
}

/// "Create a token that's a copy of Ran and Shaw, except it's not legendary." (CR 707.2, 707.9b, 111.1): a copy of the copiable values
/// of Ran and Shaw as they are, or as they last were if they have left the battlefield (CR 608.2h), with the one stated exception.
/// The token was not cast, so its own enters trigger finds nothing (CR 603.4).
fn copy_not_legendary(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let values = g.copiable_values(item.object).nonlegendary();
    g.create_token_copies_of(values, item.controller, 1)?;
    Ok(())
}

/// "Dragons you control get +2/+0 until end of turn." (CR 611.2, 611.2c): `effects::subtype_you_control_get` over Dragon, a word of
/// this card's text (CR 612).
fn dragons_get_plus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::subtype_you_control_get(g, item, Subtype::Dragon, 2, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<2>("Firebending 2"),
        triggered(
            THIS_ENTERS_IF_CAST,
            copy_not_legendary,
            "When Ran and Shaw enter, if you cast them and there are three or more Dragon and/or Lesson cards in your graveyard, create a token that's a copy of Ran and Shaw, except it's not legendary.",
        )
        .condition(three_dragon_or_lesson_cards_in_your_graveyard),
        ability(cost("{3}{R}"), dragons_get_plus_two, "{3}{R}: Dragons you control get +2/+0 until end of turn."),
    ],
    ..oracle::RAN_AND_SHAW
};
