use cantrip_api::*;
use cantrip_mechanics::statics::{base_pt, creature_of_subtype_alone};
use cantrip_mechanics::{attach::attached_object, lands, targets};
use cantrip_oracle as oracle;

// "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named
// Legitimate Businessperson. (It loses all other colors, card types, creature types, and names.)": one static ability, an effect
// in each of five layers, all at the time this Aura became attached (CR 613.7e).

/// "... named Legitimate Businessperson" (CR 612.8, layer 3): a text-changing effect that sets the name; the creature has no other name.
fn legitimate_businessperson(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.name = "Legitimate Businessperson";
    chars.name_set = true;
}

/// "... is a green and white ..." (CR 105.3, layer 5): green and white and no other color. Both are words of this card's rules text (CR 612).
fn green_and_white(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let words = view.words(source);
    chars.colors = ColorSet::of(words.color(Color::Green)).with(words.color(Color::White));
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Text, attached_object, legitimate_businessperson, "Enchanted creature is named Legitimate Businessperson."),
        static_ability(Layer::Type, attached_object, creature_of_subtype_alone::<{ lands::sub(Subtype::Citizen) }>, "Enchanted creature is a Citizen creature.").continues(),
        static_ability(Layer::Color, attached_object, green_and_white, "Enchanted creature is green and white.").continues(),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities.").continues(),
        static_ability(Layer::PtSet, attached_object, base_pt::<1, 1>, "Enchanted creature has base power and toughness 1/1.").continues(),
    ],
    ..oracle::WITNESS_PROTECTION
};
