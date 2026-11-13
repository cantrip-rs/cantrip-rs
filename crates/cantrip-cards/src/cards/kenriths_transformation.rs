use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, effects, lands, statics::{base_pt, creature_of_subtype_alone}, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

// "Enchanted creature loses all abilities and is a green Elk creature with base power and toughness 3/3. (It loses all
// other card types and creature types.)": one static ability, an effect in each of four layers, all at the time this Aura
// became attached (CR 613.7e).

/// "... is a green ..." (CR 105.3, layer 5): green and no other color. Green is a word of this card's rules text (CR 612).
fn green(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.colors = ColorSet::of(view.words(source).color(Color::Green));
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this Aura enters, draw a card.")],
    statics: &[
        static_ability(Layer::Type, attached_object, creature_of_subtype_alone::<{ lands::sub(Subtype::Elk) }>, "Enchanted creature is an Elk creature."),
        static_ability(Layer::Color, attached_object, green, "Enchanted creature is green."),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities."),
        static_ability(Layer::PtSet, attached_object, base_pt::<3, 3>, "Enchanted creature has base power and toughness 3/3."),
    ],
    ..oracle::KENRITHS_TRANSFORMATION
};
