use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// A spell with exactly one announced target, regardless of that target's current legality (CR 115.7a).
fn spell_with_single_target(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(_)) && (targets::SPELL_OR_ABILITY_WITH_A_SINGLE_TARGET.filter)(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(spell_with_single_target)], effects::change_first_target),
    ..oracle::DEFLECTION
};
