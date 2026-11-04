use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Cast this spell only during the declare attackers step. Target creature can't be blocked this turn." (CR 601.3).
fn cast_spell_only_during_declare_if(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareAttackers
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_cant_be_blocked_this_turn),
    cast_only_if: Some(cast_spell_only_during_declare_if),
    ..oracle::TELEPORT
};
