use cantrip_api::*;
use cantrip_mechanics::combat::{self, NO_POWER_TWO_OR_LESS_BLOCKERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{G}"), combat::this_gets_its_block_rule_this_turn, "{1}{G}: This creature can't be blocked by creatures with power 2 or less this turn.")],
    lasting_block_rules: &[NO_POWER_TWO_OR_LESS_BLOCKERS],
    ..oracle::VERDANT_OUTRIDER
};
