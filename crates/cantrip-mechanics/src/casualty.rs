//! Casualty N (CR 702.153a): "As an additional cost to cast this spell, you may sacrifice a creature with power N or
//! greater," and "When you cast this spell, if a casualty cost was paid for it, copy it. If the spell has any targets, you
//! may choose new targets for the copy." Paying it follows the rules for additional costs (CR 601.2b, 601.2f-h). Casualty X
//! (Ob Nixilis, the Adversary) is the same with X announced as the spell is cast (CR 107.3a, 601.2b): the spell's X, which
//! the copy copies (CR 707.10). Ob Nixilis, the Adversary (casualty X), Illicit Shipment (casualty 3) and Rob the Archives
//! (casualty 1) print it.
//!
//! A card with casualty gives `optional::<N>()` or `X` as its `optional_cost`, two labels for the announcement ("cast
//! without casualty", then paying it) as its `labels`, and `trigger(text)` among its `abilities`. A copy made with
//! exceptions ("the copy isn't legendary and has starting loyalty X") is `trigger_with(text, effect)`, its effect the card's
//! own copy, made with `Game::copy_spell_as` and the X announced, `x(q, item)`.
//!
//! Casualty here is the card's own printed instance, its one `CardDef::optional_cost`. Granted casualty (Anhelo, the
//! Painter: "The first instant or sorcery spell you cast each turn has casualty 2") is not built, and needs:
//!
//! - casualty given to a spell by another object, its cost announced and its trigger triggering as a printed one's;
//! - several instances on one spell (CR 702.153b): each paid separately, and each trigger copying for its own payment only,
//!   not for any other instance's - one optional cost and one `paid` for each;
//! - casualty X beside a kicker: the spell's X and the times it is kicked are announced apart (CR 601.2b), where today one
//!   announcement is either the kicks or the X.

use cantrip_api::*;

/// A creature with power N or greater (CR 702.153a), read as the cost is paid (CR 601.2h).
fn creature_with_power_at_least<const N: u8>(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    let chars = q.chars(id);
    chars.is(CardType::Creature) && chars.power.is_some_and(|p| p >= i16::from(N))
}

fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// Casualty N's additional cost: "you may sacrifice a creature with power N or greater".
pub const fn optional<const N: u8>() -> OptionalCost {
    OptionalCost::new(Cost::sacrifice(1, creature_with_power_at_least::<N>))
}

/// Casualty X's additional cost: "you may sacrifice a creature with power X or greater", X announced once the intention to
/// pay it is (CR 601.2b). The creature's power is read as it is sacrificed (CR 601.2h), after the mana abilities activated
/// while paying (CR 601.2g), so X may be announced up to the most a power can be (`VariableCost::SacrificeWithPowerX`).
pub const X: OptionalCost = OptionalCost::new(Cost { variable: Some(VariableCost::SacrificeWithPowerX(creature)), ..Cost::NONE });

/// "When you cast this spell, if a casualty cost was paid for it" (CR 603.4): it functions on the stack, and a cast without
/// the cost paid triggers nothing.
fn casualty_paid(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if *spell == source && q.obj(source).paid > 0 => Some(Fired { subject: Target::None, data: [0, 0] }),
        _ => None,
    }
}

/// "Copy it. If the spell has any targets, you may choose new targets for the copy" (CR 707.10, 707.10c). A copy of a
/// permanent spell becomes a token as it resolves (CR 707.10f).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_spell(item.object, item.controller, true)?;
    Ok(())
}

/// The casualty trigger; `text` is the card's own reminder text.
pub const fn trigger(text: &'static str) -> AbilityDef {
    trigger_with(text, copy_it)
}

/// The casualty trigger whose copy the card makes itself, with the exceptions it prints (CR 707.9b): Ob Nixilis, the
/// Adversary's "The copy isn't legendary and has starting loyalty X" copies with `Game::copy_spell_as`.
pub const fn trigger_with(text: &'static str, copy: Effect) -> AbilityDef {
    triggered(when_this_spell_is_cast(casualty_paid), copy, text)
}

/// The X announced for the spell whose casualty trigger this is (CR 107.3a): the spell's, on the stack or as it last was
/// there once it has left (CR 608.2h), as the copy copies it (CR 707.10).
pub fn x(q: &dyn Query, item: &StackItem) -> u16 {
    let s = q.state();
    s.stack.iter().find(|i| i.kind == StackKind::Spell && i.object == item.object).or_else(|| s.left_stack(item.object)).map_or(0, |spell| spell.x)
}
