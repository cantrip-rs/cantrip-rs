use cantrip_api::*;
use cantrip_mechanics::replacements::instead_of_drawing;
use cantrip_oracle as oracle;

/// "Players play with their hands revealed. If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card." (CR 608.2).
fn hand_revealed_0(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}

/// "If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card." (CR 614.1a).
fn if_player_would_draw_card_applies(
    _: &dyn Query,
    _: &Asker,
    proposal: &Proposal,
) -> bool {
    matches!(proposal, Proposal::Draw(_))
}

/// "If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card." (CR 608.2, 121.1, 701.20a).
fn if_player_would_draw_card_then_steps(
    g: &mut dyn Game,
    asker: &Asker,
    subject: Target,
    _: [u32; 2],
) -> Result<(), Suspend> {
    let Some(player) = subject.player() else { return Ok(()) };
    // The revealed object is the top of the library, and the payment refers to that same object.
    let revealed = g.state().zone(Zone::library(player)).first().copied();
    if let Some(card) = revealed {
        g.begin_effect_reveal(player, &[card]);
    }
    let others: Vec<PlayerId> = g.state().apnap().filter(|&p| p != player).collect();
    for payer in others {
        if g.may_pay(payer, asker.source, asker.source, life(2), &PAY_QUESTION)? {
            if let Some(card) = revealed {
                // The zone change ends the reveal of the old object, including a library redirection.
                g.put_into_graveyard(&[card]);
            }
            return Ok(());
        }
    }
    g.draw(player, 1);
    if let Some(card) = revealed { g.end_effect_reveal(&[card]); }
    Ok(())
}

/// "If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card." (CR 614.1a).
fn if_player_would_draw_card_then(
    g: &mut dyn Game,
    asker: &Asker,
    before: &Proposal,
    _: &Proposal,
) {
    let Proposal::Draw(d) = before else { return };
    let subject = Target::Player(d.player);
    let data = [1u32, u32::MAX];
    let _ = if_player_would_draw_card_then_steps(g, asker, subject, data);
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost} to put the revealed card into its owner’s graveyard?",
    "If no player pays, that player draws a card",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    replacements: &[
        replacement(
                ReplacementKind::Draw,
                if_player_would_draw_card_applies,
                instead_of_drawing,
                "If a player would draw a card, they reveal it instead. Then any other player may pay 2 life. If a player does, put that card into its owner's graveyard. Otherwise, that player draws a card.",
            )
            .then(if_player_would_draw_card_then),
    ],
    play_rules: &[PlayRule::HandRevealed(hand_revealed_0)],
    ..oracle::ZURS_WEIRDING
};
