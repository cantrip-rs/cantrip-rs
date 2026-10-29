use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::base_pt, targets};
use cantrip_oracle as oracle;

// "Enchanted creature is a Turtle with base power and toughness 0/1. It can't attack and loses all abilities.": static
// abilities of the Aura, in layers 4, 6 and 7b, all at the time it became attached (CR 613.7e), and a rule of attacking.

/// "... is a Turtle" (CR 205.1a, layer 4): its creature types become Turtle alone; its card types, its other subtypes and its
/// colors stay. A changeling's every creature type is overwritten. Turtle is a word of this card's rules text (CR 612).
fn turtle(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let kept = chars.subtypes.iter().filter(|t| !t.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with);
    chars.subtypes = kept.with(view.words(source).subtype(Subtype::Turtle));
}

/// "Enchanted creature can't attack" (CR 508.1c): asked of each creature that could be declared an attacker, with this Aura as
/// the source. The Aura keeps its own ability whatever the creature loses.
fn enchanted_creature_cant_attack(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    q.obj(source).attached_to != Some(attacker)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[
        static_ability(Layer::Type, attached_object, turtle, "Enchanted creature is a Turtle."),
        static_ability(Layer::PtSet, attached_object, base_pt::<0, 1>, "Enchanted creature has base power and toughness 0/1."),
        loses_all_abilities(attached_object, "Enchanted creature loses all abilities."),
    ],
    attack_rules: &[enchanted_creature_cant_attack],
    ..oracle::RETRO_MUTATION
};
