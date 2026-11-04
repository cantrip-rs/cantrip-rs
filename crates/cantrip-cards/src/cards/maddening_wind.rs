use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{G}")), "Cumulative upkeep {G} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"),
        effects::enchanted_permanents_upkeep_damage::<2>(
            "At the beginning of the upkeep of enchanted creature's controller, this Aura deals 2 damage to that player.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::MADDENING_WIND
};
