use cantrip_api::*;
use cantrip_mechanics::library::{Find, search_to_hand};
use cantrip_mechanics::{class, discard_at_random, effects, triggers};
use cantrip_oracle as oracle;

/// Level 2: "Whenever you discard a card", once the Class is level 2 or more (CR 716.2a): the ability exists only then.
fn you_discard_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 2 { triggers::you_discard(q, me, event) } else { None }
}

/// "Whenever you discard a card, this Class deals 2 damage to each opponent."
fn two_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 2);
    Ok(())
}

/// "Search your library for a card, put it into your hand, shuffle, then discard a card at random" (CR 701.23, 701.9b):
/// the card found may be the one discarded, and the discard is one more discard for the level 2 ability.
fn search_then_discard_at_random(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_hand(g, item.controller, item.object, Find::ANY, |_, _| true)?;
    discard_at_random(g, item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOU_ATTACK, |g, item| effects::may_discard_to_draw(g, item, 1), "Whenever you attack, you may discard a card. If you do, draw a card."),
        class::level_two("{1}{R}", "{1}{R}: Level 2"),
        triggered(when_asked(EventMask::DISCARDED, you_discard_at_level_two), two_damage_to_each_opponent, "Whenever you discard a card, this Class deals 2 damage to each opponent."),
        class::level_three("{1}{R}", "{1}{R}: Level 3"),
        triggered(
            class::BECOMES_LEVEL_THREE,
            search_then_discard_at_random,
            "When this Class becomes level 3, search your library for a card, put it into your hand, shuffle, then discard a card at random.",
        ),
    ],
    ..oracle::COOL_BUT_RUDE
};
