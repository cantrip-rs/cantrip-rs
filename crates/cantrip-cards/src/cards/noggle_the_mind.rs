use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, colorless, creature_type_alone};
use cantrip_mechanics::{attach::attached_object, lands, targets};
use cantrip_oracle as oracle;

// "Enchanted creature loses all abilities and is a colorless Noggle with base power and toughness 1/1. (It loses all
// colors and all other creature types.)": one static ability, an effect in layers 4, 5, 6 and 7b, all at the time this Aura
// became attached (CR 613.7e).

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Type, attached_object, creature_type_alone::<{ lands::sub(Subtype::Noggle) }>, "Enchanted creature is a Noggle."),
        static_ability(Layer::Color, attached_object, colorless, "Enchanted creature is colorless."),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities."),
        static_ability(Layer::PtSet, attached_object, base_pt::<1, 1>, "Enchanted creature has base power and toughness 1/1."),
    ],
    ..oracle::NOGGLE_THE_MIND
};
