use cantrip_api::*;
use cantrip_mechanics::{cards_at_random_from_hand, targets};
use cantrip_oracle as oracle;

/// "{3}, {T}: Target player reveals a card at random from their hand. If it's a land card, that player discards it unless they pay 1 life. If it isn't a land card, the player discards it unless they pay life equal to its mana value. Activate only during your turn." (CR 608.2, 701.9a, 701.20a).
fn target_player_reveals_card_random(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let shown = cards_at_random_from_hand(g, player, 1);
    let Some(&card) = shown.first() else { return Ok(()) };
    g.reveal(player, &shown);
    g.end_sentence();

    let life_cost = if g.chars(card).is(CardType::Land) { 1 } else { g.mana_value(card) };
    if !g.may_pay(player, item.object, card, Cost { life: life_cost, ..Cost::NONE }, &PAY_QUESTION)? {
        g.discard_revealed(&shown);
    }
    g.end_sentence();
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}, or discard {subject}?",
    "",
    "Pay {cost}",
    "Discard {subject}",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{3}").and(TAP),
                target_player_reveals_card_random,
                "{3}, {T}: Target player reveals a card at random from their hand. If it's a land card, that player discards it unless they pay 1 life. If it isn't a land card, the player discards it unless they pay life equal to its mana value. Activate only during your turn.",
            )
            .targets(&[targets::PLAYER])
            .timing(Timing::OwnTurn),
    ],
    ..oracle::WAND_OF_ITH
};
