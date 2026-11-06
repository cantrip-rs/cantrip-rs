use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// An instant or Aura spell targeting a permanent controlled by this ability's controller (CR 115.1, 608.2b).
fn eligible(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    let Some(item) = q.state().stack.iter().find(|i| i.kind == StackKind::Spell && i.object == id) else { return false };
    (q.chars(id).is(CardType::Instant) || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura)))
        && targets::spell_targets_permanent_controlled_by(q, item, asker.controller)
}

/// Counter the spell if it remains a legal target (CR 701.6a, 608.2b).
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}").and(TAP), counter,
        "{3}, {T}: Counter target instant or Aura spell that targets a permanent you control.")
        .targets(&[target(eligible)])],
    ..oracle::RING_OF_IMMORTALS
};
