use cantrip_api::*;
use cantrip_mechanics::{combat, delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                delayed::target_creature_gets_then_sacrifice_this_when_it_leaves::<2, 2>,
                "{T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat.",
            )
            .targets(&[targets::CREATURE])
            .condition(combat::activate_only_during_combat),
    ],
    delayed: &[
        triggered(
            triggers::THIS_LEAVES,
            effects::sacrifice_self,
            "When that creature leaves the battlefield this turn, sacrifice this creature.",
        ),
    ],
    ..oracle::KJELDORAN_ELITE_GUARD
};
