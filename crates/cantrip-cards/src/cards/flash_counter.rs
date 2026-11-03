use cantrip_api::*;
use cantrip_mechanics::{counter_target, targets};
use cantrip_oracle as oracle;

/// "Target instant spell" (CR 115.1a, 112.1).
fn instant_spell(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::SPELL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Instant))
}

/// "Counter target instant spell" (CR 701.6a, 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(instant_spell)], effect),
    ..oracle::FLASH_COUNTER
};
