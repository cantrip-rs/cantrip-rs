use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_object, enchanted_creature_cant_block, exile_enchanted_creature}, lands, statics::creature_type_alone, targets, triggers};
use cantrip_oracle as oracle;

/// "Enchanted creature gets -4/-0" (CR 613.4c, layer 7c).
fn minus_four_power(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|power| power - 4);
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        triggers::EACH_UPKEEP,
        exile_enchanted_creature,
        "At the beginning of each upkeep, if there are seven or more cards in your graveyard, exile enchanted creature.",
    )
    .condition(cantrip_mechanics::you_have_threshold)],
    block_rules: &[enchanted_creature_cant_block],
    statics: &[
        static_ability(Layer::Type, attached_object, creature_type_alone::<{ lands::sub(Subtype::Salamander) }>, "Enchanted creature is a Salamander."),
        static_ability(Layer::PtModify, attached_object, minus_four_power, "Enchanted creature gets -4/-0.").continues(),
    ],
    ..oracle::DE_EVOLVE
};
