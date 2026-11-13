use cantrip_api::*;
use cantrip_mechanics::{combat, targets};
use cantrip_oracle as oracle;

/// A blocked attacking creature (CR 115.1c, 509.1h).
fn blocked_attacker(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.chars(id).is(CardType::Creature) && q.is_attacking(id) && q.is_blocked(id))
}

/// Exchange the blockers if both targets are legal and the new blocks are possible (CR 608.2b).
fn exchange_blockers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let first = targets::object(g, item, 0);
    let second = targets::object(g, item, 1);
    combat::swap_blocks(g, first, second, true);
    Ok(())
}

/// Activate only during the declare blockers step (CR 602.5).
fn during_blockers(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareBlockers
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            exchange_blockers,
            "{T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step.",
        )
        .targets(&[target(blocked_attacker).between(2, 2)])
        .condition(during_blockers),
    ],
    ..oracle::GENERAL_JARKELD
};
