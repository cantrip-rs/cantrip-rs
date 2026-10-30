//! Blight (CR 701.68a): "to blight N means to put N -1/-1 counters on a creature you control", as a cost. A spell's additional
//! cost ("as an additional cost to cast this spell, you may blight 1": Requiting Hex, Pyrrhic Strike; Cinder Strike and Burning
//! Curiosity print it as an optional cost, and Wild Unraveling and Bogslither's Embrace as one of two ways to pay), or a part of
//! an activated ability's cost ("{T}, Blight 1: Surveil 1": Dawnhand Dissident; Gristle Glutton and Sting-Slinger print it too).
//! A spell or ability that blights as it resolves ("you may blight 2. If you do, ...") uses `in_effect` or `may_in_effect`.
//! Rule 701.68d ("whenever a player blights") has no event yet.
//!
//! Choosing the creature and putting the counters is the core's (`Cost::blight`, CR 601.2h by way of 602.2b for an ability); a
//! player who controls no creature can't choose to blight (CR 701.68b). A card with the optional kind gives `optional(n)` as its
//! `optional_cost`, and "if this spell's additional cost was paid" is `blighted` of the resolving spell; an ability puts
//! `cost(n)` into its cost with `.and`.

use cantrip_api::*;

/// The creatures `player` may blight: those they control that can have -1/-1 counters put on them.
fn candidates(g: &dyn Game, player: PlayerId) -> Vec<ObjectId> {
    g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature) && !g.counters_banned(id, player, CounterKind::MinusOne)).collect()
}

/// "Blight N" as an effect resolves (CR 701.68a, 608.2): `player` chooses a creature they control that can have -1/-1 counters
/// put on it (CR 101.2: Melira) and the N counters go on it, put by the player whose effect it is (CR 122.6a). Always asked,
/// even when the choice is forced. A player who controls no such creature can't blight (CR 701.68b): nothing is asked and the
/// answer is None; otherwise the creature that was blighted, for "if you do" and "the blighted creature". Blighted Blackthorn,
/// Dream Seizer and Champion of the Weird print it.
pub fn in_effect(g: &mut dyn Game, player: PlayerId, source: ObjectId, n: u8) -> Result<Option<ObjectId>, Suspend> {
    let eligible = candidates(g, player);
    let chosen = crate::choose::cards_from_kind(g, player, source, Purpose::Blight, "creature", eligible.clone(), eligible, 1, false)?;
    let Some(&creature) = chosen.first() else { return Ok(None) };
    g.add_counters(creature, CounterKind::MinusOne, i16::from(n));
    Ok(Some(creature))
}

/// "You may blight N" as an effect resolves: `in_effect` after `question` is answered yes. Not offered when the player has no
/// creature to blight (CR 701.68b). The creature that was blighted, if they did.
pub fn may_in_effect(g: &mut dyn Game, player: PlayerId, source: ObjectId, n: u8, question: &'static Question) -> Result<Option<ObjectId>, Suspend> {
    if candidates(g, player).is_empty() || !crate::effects::ask_may(g, player, source, Purpose::Ask(question))? {
        return Ok(None);
    }
    in_effect(g, player, source, n)
}

/// The cost "blight N" as a part of a cost.
pub const fn cost(n: u8) -> Cost {
    Cost { blight: n, ..Cost::NONE }
}

/// "As an additional cost to cast this spell, you may blight N."
pub const fn optional(n: u8) -> OptionalCost {
    OptionalCost { cost: cost(n) }
}

/// "If this spell's additional cost was paid": it was cast with its optional cost paid. A copy of the spell has it paid
/// when the original did (CR 707.10).
pub fn blighted(item: &StackItem) -> bool {
    item.paid > 0
}
