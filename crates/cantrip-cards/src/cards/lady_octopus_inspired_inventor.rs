use cantrip_api::*;
use cantrip_mechanics::{choose, history, targets};
use cantrip_oracle as oracle;

/// "Whenever you draw your first or second card each turn" (CR 120.3, 603.2): a draw by Lady Octopus's controller who had
/// drawn fewer than two cards this turn before it (the history holds the draws before the one being matched), in any step.
fn first_or_second_draw(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Drew { player, .. } if *player == q.controller(source) && history::drawn_this_turn(q, *player) < 2 => Fired::PLAIN,
        _ => None,
    }
}

/// "Put an ingenuity counter on Lady Octopus", if she is still on the battlefield (CR 400.7).
fn ingenuity(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::Ingenuity, 1);
    }
    Ok(())
}

/// "You may cast an artifact spell from your hand with mana value less than or equal to the number of ingenuity counters
/// on Lady Octopus without paying its mana cost." The spell is cast as the ability resolves or not at all (CR 608.2g, the
/// rulings); what it is - an artifact, its mana value - is the spell the card would be cast as (CR 601.3e, 709.3b), X being 0
/// (CR 107.3b, 118.9). The counters are counted as she is now, or as she last was on the battlefield (CR 608.2h).
fn cast_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let most = g.obj(item.object).counters(CounterKind::Ingenuity).max(0) as u32;
    choose::may_cast_from_hand_free(g, you, item.object, "artifact", |g, card, w| g.chars_as_cast(card, w.way, you).is(CardType::Artifact) && w.mana_value <= most)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::DREW, first_or_second_draw),
            ingenuity,
            "Whenever you draw your first or second card each turn, put an ingenuity counter on Lady Octopus.",
        ),
        ability(
            TAP,
            cast_artifact,
            "{T}: You may cast an artifact spell from your hand with mana value less than or equal to the number of ingenuity counters on Lady Octopus without paying its mana cost.",
        ),
    ],
    ..oracle::LADY_OCTOPUS_INSPIRED_INVENTOR
};
