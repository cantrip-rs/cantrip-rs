use cantrip_api::*;
use cantrip_mechanics::statics::base_pt;
use cantrip_mechanics::{attach::attached_object, targets};
use cantrip_oracle as oracle;

// "Enchanted creature loses all abilities and is a colorless Noggle with base power and toughness 1/1. (It loses all
// colors and all other creature types.)": one static ability, an effect in layers 4, 5, 6 and 7b, all at the time this Aura
// became attached (CR 613.7e).

/// "... is a ... Noggle" (CR 205.1a, 613.1d, layer 4): its creature types become Noggle alone; its card types, its other
/// subtypes and its supertypes stay. Noggle is a word of this card's rules text (CR 612).
fn noggle(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let kept = chars.subtypes.iter().filter(|t| !t.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with);
    chars.subtypes = kept.with(view.words(source).subtype(Subtype::Noggle));
}

/// "... is a colorless ..." (CR 105.3, layer 5): it has no color.
fn colorless(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.colors = ColorSet::NONE;
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Type, attached_object, noggle, "Enchanted creature is a Noggle."),
        static_ability(Layer::Color, attached_object, colorless, "Enchanted creature is colorless."),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities."),
        static_ability(Layer::PtSet, attached_object, base_pt::<1, 1>, "Enchanted creature has base power and toughness 1/1."),
    ],
    ..oracle::NOGGLE_THE_MIND
};
