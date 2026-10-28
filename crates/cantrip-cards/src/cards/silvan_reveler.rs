use cantrip_api::*;
use cantrip_mechanics::{choose_from_hand, hand, triggers};
use cantrip_oracle as oracle;

const RETURN: Question = Question::new("ReturnToHand", "pay {cost} to return {subject} from your graveyard to your hand?", "", "Pay {cost}", "Leave it");

/// "Draw a card, then discard a card. If you discard a land card this way, put it from your graveyard onto the battlefield tapped"
/// (CR 121.1, 701.9, 701.9c, 110.2a): the card is drawn before the discard is chosen, so it may be the one discarded. A land card is
/// judged by the discard's record of what it was where it went (`hand::discarded_chars`; a card a replacement put somewhere hidden
/// is undefined and no land card), and it is put onto the battlefield, tapped and under its owner's control, only if it really is
/// in the graveyard (CR 400.7): a land card that went somewhere else, or has since left, stays where it is.
fn loot_and_put_a_discarded_land_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.draw(you, 1);
    g.end_sentence();
    let chosen = choose_from_hand(g, you, item.object, 1)?;
    if let Some(&card) = g.discard_together(&chosen).first()
        && hand::discarded_chars(&*g, card).is_some_and(|chars| chars.is(CardType::Land))
        && g.obj(card).zone == Zone::graveyard(you)
    {
        g.put_onto_battlefield(card, you, true)?;
    }
    Ok(())
}

/// "You may pay {1}{G}{U}. If you do, return this card from your graveyard to your hand" (CR 118.12, 603.4): the payment is chosen as
/// the ability resolves; once this card is no longer the object that triggered (CR 400.7) nothing is asked and nothing returns.
fn pay_to_return_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) && g.may_pay(item.controller, item.object, item.object, cost("{1}{G}{U}"), &RETURN)? {
        g.return_to_hand(&[item.object]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            loot_and_put_a_discarded_land_onto_the_battlefield,
            "When this creature enters, draw a card, then discard a card. If you discard a land card this way, put it from your graveyard onto the battlefield tapped.",
        ),
        triggered(
            triggers::LANDFALL,
            pay_to_return_to_hand,
            "Landfall \u{2014} Whenever a land you control enters, you may pay {1}{G}{U}. If you do, return this card from your graveyard to your hand.",
        )
        // It returns this card from the graveyard, so it functions only there (CR 113.6m).
        .functions_in(IN_A_GRAVEYARD),
    ],
    ..oracle::SILVAN_REVELER
};
