use cantrip_api::*;
use cantrip_mechanics::triggers::{modified_creature_you_control_deals_combat_damage_to_player, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::plus_one_counter_on_target, "When SP//dr enters, put a +1/+1 counter on target creature.").targets(&[targets::CREATURE]),
        triggered(
            when_asked(EventMask::DAMAGE, modified_creature_you_control_deals_combat_damage_to_player),
            |g, item| effects::draw(g, item, 1),
            "Whenever a modified creature you control deals combat damage to a player, draw a card. (Equipment, Auras you control, and counters are modifications.)",
        ),
    ],
    ..oracle::SP_DR_PILOTED_BY_PENI
};
