use cantrip_api::*;
use cantrip_mechanics::combat::{self, EXCEPT_BY_HASTE};
use cantrip_mechanics::protection;
use cantrip_oracle as oracle;

/// Haste is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}"),
        combat::this_gets_its_block_rule_this_turn,
        "{3}: This creature can't be blocked this turn except by creatures with haste.",
    )],
    statics: &[protection::from_subtype::<{ Subtype::Coyote as u16 }>("Protection from Coyotes")],
    lasting_block_rules: &[EXCEPT_BY_HASTE],
    ..oracle::RESILIENT_ROADRUNNER
};
