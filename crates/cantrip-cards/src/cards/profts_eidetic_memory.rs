use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::{history::drawn_this_turn, targets, triggers};
use cantrip_oracle as oracle;

/// The intervening "if you've drawn more than one card this turn" (CR 603.4): asked as it triggers and as it resolves.
fn drawn_more_than_one(q: &dyn Query, asker: &Asker) -> bool {
    drawn_this_turn(q, asker.controller) > 1
}

/// Put X +1/+1 counters on target creature you control, X being the cards you've drawn this turn minus one, counted as the
/// ability resolves (CR 608.2h).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = drawn_this_turn(g, item.controller) as i16 - 1;
    if x > 0
        && let Some(creature) = targets::object(g, item, 0)
    {
        g.add_counters(creature, CounterKind::PlusOne, x);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When Proft's Eidetic Memory enters, draw a card."),
        triggered(triggers::BEGINNING_OF_YOUR_COMBAT, grow, "At the beginning of combat on your turn, if you've drawn more than one card this turn, put X +1/+1 counters on target creature you control, where X is the number of cards you've drawn this turn minus one.")
            .targets(&[targets::CREATURE_YOU_CONTROL])
            .condition(drawn_more_than_one),
    ],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::PROFTS_EIDETIC_MEMORY
};
