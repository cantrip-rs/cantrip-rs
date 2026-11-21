use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever you sacrifice a permanent during your turn" (CR 701.21a, 603.2): a sacrifice of yours while you are the active player.
fn you_sacrifice_a_permanent_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.state().turn.active == me.controller { triggers::you_sacrifice_a_permanent(q, me, event) } else { None }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            |g, item| tokens::create(g, item.controller, &tokens::CLUE, 1).map(|_| ()),
            "When this enchantment enters, create a Clue token. (It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, you_sacrifice_a_permanent_during_your_turn),
            tokens::create_an_ally,
            "Whenever you sacrifice a permanent during your turn, create a 1/1 white Ally creature token. This ability triggers only once each turn.",
        )
        .on_sacrifice()
        .triggers_each_turn(1),
    ],
    ..oracle::TOLLS_OF_WAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE, &tokens::ALLY];
