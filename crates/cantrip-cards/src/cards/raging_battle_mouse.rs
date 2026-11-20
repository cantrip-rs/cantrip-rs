use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{cost_reduction, effects, history, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::second_spell_you_cast_each_turn_costs_less::<1>],
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::target_creature_gets::<1, 1>,
        "Celebration — At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your control this turn, target creature you control gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .condition(history::you_celebrate)],
    ..oracle::RAGING_BATTLE_MOUSE
};
