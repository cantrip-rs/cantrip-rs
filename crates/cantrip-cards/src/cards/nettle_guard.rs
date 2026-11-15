use cantrip_api::*;
use cantrip_mechanics::effects::{sacrifice_to_destroy_artifact_or_enchantment, this_gets};
use cantrip_mechanics::triggers::VALIANT;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            VALIANT,
            this_gets::<0, 2>,
            "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, it gets +0/+2 until end of turn.",
        ),
        sacrifice_to_destroy_artifact_or_enchantment(1, "{1}, Sacrifice this creature: Destroy target artifact or enchantment."),
    ],
    ..oracle::NETTLE_GUARD
};
