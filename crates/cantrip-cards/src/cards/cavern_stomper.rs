use cantrip_api::*;
use cantrip_mechanics::combat::{self, NO_POWER_TWO_OR_LESS_BLOCKERS};
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::scry_two, "When this creature enters, scry 2."),
        ability(cost("{3}{G}"), combat::this_gets_its_block_rule_this_turn, "{3}{G}: This creature can't be blocked by creatures with power 2 or less this turn."),
    ],
    lasting_block_rules: &[NO_POWER_TWO_OR_LESS_BLOCKERS],
    ..oracle::CAVERN_STOMPER
};
