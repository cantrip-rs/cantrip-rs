use cantrip_api::*;
use cantrip_mechanics::triggers::you_cast_a_spell_with_mana_value_at_least;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715), Storm Reading: "Draw four cards, then discard two cards." (CR 121.1, 701.9).
pub static STORM_READING: CardDef = CardDef { spell: spell(&[], effects::draw_then_discard::<4, 2>), ..oracle::STORM_READING };

/// "Whenever you cast a spell with mana value 5 or greater, tap target creature an opponent controls and put a stun counter on it."
/// (CR 603.2, 701.26a, 122.1d): the mana value is the spell's on the stack (CR 202.3e), so an Adventure cast is judged by its own cost;
/// the trigger sees the cast, it is not itself cast.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_with_mana_value_at_least::<5>),
        effects::tap_and_stun::<1>,
        "Whenever you cast a spell with mana value 5 or greater, tap target creature an opponent controls and put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    adventure: Some(&STORM_READING),
    ..oracle::GALVANIC_GIANT
};

pub static EXTRA: &[&CardDef] = &[&STORM_READING];
