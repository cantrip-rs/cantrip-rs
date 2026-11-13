use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, creature_of_subtype_alone};
use cantrip_mechanics::{attach::attached_object, lands, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature ... is a black ... creature" (CR 105.3, layer 5): black and no other color.
fn black(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.colors = view.words(source).colors(ColorSet::of(Color::Black));
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Type, attached_object, creature_of_subtype_alone::<{ lands::sub(Subtype::Skeleton) }>, "Enchanted creature is a Skeleton creature."),
        static_ability(Layer::Color, attached_object, black, "Enchanted creature is black.").continues(),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities.").continues(),
        static_ability(Layer::PtSet, attached_object, base_pt::<1, 1>, "Enchanted creature has base power and toughness 1/1.").continues(),
    ],
    ..oracle::EATEN_BY_PIRANHAS
};
