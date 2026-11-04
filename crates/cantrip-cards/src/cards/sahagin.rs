use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature and it can't be blocked this turn" (CR 122.1, 509.1b, 611.2): both only while it is
/// still the creature on the battlefield (CR 400.7); it resolves whether or not the spell that caused it still exists.
fn grow_and_cant_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
        effects::this_cant_be_blocked_this_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::noncreature_spell_you_cast_with_at_least_mana_spent::<4>),
        grow_and_cant_be_blocked,
        "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature and it can't be blocked this turn.",
    )],
    ..oracle::SAHAGIN
};
