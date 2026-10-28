use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers, prepare};
use cantrip_oracle as oracle;
/// Opponent-controlled creature, with current mana value at most three (CR 202.3, 115.1).
fn small_opposing(q: &dyn Query, me: &Asker, t: Target) -> bool {
 (targets::CREATURE_AN_OPPONENT_CONTROLS.filter)(q, me, t) && matches!(t, Target::Object(id) if q.mana_value(id) <= 3)
}
/// Bounce, then surveil; illegal targeting is handled by the core (CR 608.2b-c).
fn unwind(g: &mut dyn Game, i: &StackItem) -> Result<(), Suspend> {
 effects::return_first_target_to_hand(g, i)?;
 effects::surveil(g, i, 1)
}
static UNWIND_HISTORY: CardDef = CardDef {
 name: "Unwind History", cost: mana("{1}{U}"), types: TypeSet::of(&[CardType::Sorcery]),
 spell: spell(&[target(small_opposing)], unwind),
 text: "Return target creature an opponent controls with mana value 3 or less to its owner's hand. Surveil 1.", ..CardDef::BLANK
};
pub static CARD: CardDef = CardDef {
 enters: Some(prepare::enters_prepared), prepare: Some(&UNWIND_HISTORY),
 abilities: &[triggered(triggers::YOU_SCRY_OR_SURVEIL, effects::this_gets::<1, 1>, "Whenever you scry or surveil, this creature gets +1/+1 until end of turn.")],
 ..oracle::DIVINER_OF_VICTORY
};
pub static EXTRA: &[&CardDef] = &[&UNWIND_HISTORY];
