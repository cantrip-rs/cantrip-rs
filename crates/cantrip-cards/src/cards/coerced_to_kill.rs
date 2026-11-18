use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, lands, statics, targets};
use cantrip_oracle as oracle;

const TEXT: &str = "Enchanted creature has base power and toughness 1/1, has deathtouch, and is an Assassin in addition to its other types.";

/// "Enchant creature" (CR 303.4a), "You control enchanted creature." (CR 613.1b, layer 2) and one effect across three layers (CR 613.6): it is an
/// Assassin besides its other types (layer 4), has deathtouch (layer 6) and has base power and toughness 1/1 (layer 7b), so counters and other
/// effects that change power and toughness (layer 7c) still apply to the 1/1.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        control_changing(attached_object, "You control enchanted creature."),
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Assassin) }>, TEXT),
        static_ability(Layer::Ability, attached_object, statics::have_deathtouch, TEXT).continues(),
        static_ability(Layer::PtSet, attached_object, statics::base_pt::<1, 1>, TEXT).continues(),
    ],
    ..oracle::COERCED_TO_KILL
};
