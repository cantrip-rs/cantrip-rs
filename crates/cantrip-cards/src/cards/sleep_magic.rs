use cantrip_api::*;
use cantrip_mechanics::{
    attach::{attached_to_it, tap_enchanted},
    effects, targets,
    triggers::{ENCHANTED_CREATURE_DEALT_DAMAGE, THIS_ENTERS},
};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        triggered(ENCHANTED_CREATURE_DEALT_DAMAGE, effects::sacrifice_self, "When enchanted creature is dealt damage, sacrifice this Aura.")
            .sums_per_subject(),
    ],
    untap_bans: &[attached_to_it],
    ..oracle::SLEEP_MAGIC
};
