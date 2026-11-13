use cantrip_api::*;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// "Play with your hand revealed. If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card. Whenever a creature is put into your graveyard from the battlefield, return it to your hand." (CR 604.1).
fn hand_revealed_0(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller
}

/// "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card." (CR 614.1a).
fn if_would_draw_card_reveal_applies(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Draw(d) if d.player == asker.controller)
}

/// "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card." (CR 614.1a).
fn if_would_draw_card_reveal_apply(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    let Proposal::Draw(d) = proposal else { return };
    d.replaced = true;
}

/// "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card." (CR 614.1a, 614.5, 121.1, 701.20a): the draw that
/// follows is part of the replaced event, so this effect does not apply to it again.
fn if_would_draw_card_reveal_then(g: &mut dyn Game, _: &Asker, before: &Proposal, _: &Proposal) {
    let Proposal::Draw(d) = before else { return };
    let top = top_of_library(g, d.player, 1);
    g.reveal(d.player, &top);
    if top.first().is_some_and(|&card| g.chars(card).is(CardType::Creature)) {
        g.put_into_graveyard(&top);
    } else {
        g.draw(d.player, 1);
    }
}

/// "Whenever a creature is put into your graveyard from the battlefield" (CR 603.2).
fn when_creature_put_into_graveyard_battlefield(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    if from.is(ZoneKind::Battlefield)
        && *to == Zone::graveyard(asker.controller)
        && q.chars(*old).is(CardType::Creature)
    {
        return Fired::about(Target::Object(*old));
    }
    None
}

/// "Whenever a creature is put into your graveyard from the battlefield, return it to your hand." (CR 608.2, 400.7).
fn return_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let cards: Vec<ObjectId> = g.found_all_after_move(creature).into_iter()
        .filter(|&card| g.obj(card).zone == Zone::graveyard(item.controller))
        .collect();
    g.return_to_hand(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::ZONE_CHANGE,
                when_creature_put_into_graveyard_battlefield,
            ),
            return_hand,
            "Whenever a creature is put into your graveyard from the battlefield, return it to your hand.",
        ),
    ],
    replacements: &[
        replacement(
                ReplacementKind::Draw,
                if_would_draw_card_reveal_applies,
                if_would_draw_card_reveal_apply,
                "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card.",
            )
            .then(if_would_draw_card_reveal_then),
    ],
    play_rules: &[PlayRule::HandRevealed(hand_revealed_0)],
    ..oracle::ENDURING_RENEWAL
};
