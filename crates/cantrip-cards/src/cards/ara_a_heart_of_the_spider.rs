use cantrip_api::*;
use cantrip_mechanics::triggers::{modified_creature_you_control_deals_combat_damage_to_player, YOU_ATTACK};
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_ATTACK, effects::plus_one_counter_on_target, "Whenever you attack, put a +1/+1 counter on target attacking creature.")
            .targets(&[targets::ATTACKING_CREATURE]),
        triggered(
            when_asked(EventMask::DAMAGE, modified_creature_you_control_deals_combat_damage_to_player),
            library::exile_top_card_play_this_turn,
            "Whenever a modified creature you control deals combat damage to a player, exile the top card of your library. You may play that card this turn. (Equipment, Auras you control, and counters are modifications.)",
        ),
    ],
    ..oracle::ARA_A_HEART_OF_THE_SPIDER
};
