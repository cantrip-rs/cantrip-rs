use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Put eight +1/+1 counters on Ultros" (CR 122.1, 400.7): only while it is still the creature on the battlefield; it resolves
/// whether or not the spell that caused it still exists.
fn eight_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if is_live(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 8);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::noncreature_spell_you_cast_with_at_least_mana_spent::<4>),
            effects::tap_and_stun::<1>,
            "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, tap target creature an opponent controls and put a stun counter on it.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::noncreature_spell_you_cast_with_at_least_mana_spent::<8>),
            eight_counters,
            "Whenever you cast a noncreature spell, if at least eight mana was spent to cast it, put eight +1/+1 counters on Ultros.",
        ),
    ],
    ..oracle::ULTROS_OBNOXIOUS_OCTOPUS
};
