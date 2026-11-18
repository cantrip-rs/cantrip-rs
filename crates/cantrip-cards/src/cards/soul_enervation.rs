use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::target_creature_gets::<-4, -4>,
            "When this enchantment enters, target creature gets -4/-4 until end of turn.",
        )
        .targets(&[targets::CREATURE]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::creature_card_leaves_your_graveyard),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever one or more creature cards leave your graveyard, each opponent loses 1 life and you gain 1 life.",
        )
        .once_per_batch(),
    ],
    ..oracle::SOUL_ENERVATION
};
