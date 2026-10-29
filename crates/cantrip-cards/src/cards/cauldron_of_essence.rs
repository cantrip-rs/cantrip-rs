use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_DIES;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life" (CR 700.4, 603.10a): once for each creature,
/// including those dying as the Cauldron leaves. The ability: the target is chosen before the creature is sacrificed as a cost
/// (CR 601.2c, 601.2h), so that creature is never a legal one.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            CREATURE_YOU_CONTROL_DIES,
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life.",
        ),
        ability(
            cost("{1}{B}{G}, {T}").and(sacrifice(1, filters::creature)),
            effects::return_first_target_to_battlefield,
            "{1}{B}{G}, {T}, Sacrifice a creature: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.",
        )
        .sorcery_speed()
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::CAULDRON_OF_ESSENCE
};
