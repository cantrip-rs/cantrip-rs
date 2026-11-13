use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::{colorless, grant_delayed_ability}, targets};
use cantrip_oracle as oracle;

/// "Enchant creature, land, or planeswalker" (CR 303.4, 205.2a): a permanent that is any of the three types.
fn creature_land_or_planeswalker(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && {
        let chars = q.chars(id);
        chars.is(CardType::Creature) || chars.is(CardType::Land) || chars.is(CardType::Planeswalker)
    })
}

/// "Enchanted permanent is a ... land ... and loses all other card types" (CR 205.1a, 613.1d, layer 4): its card types become
/// land alone, and the subtypes of the types it lost go with them; its land types stay (the ruling) and so do its supertypes.
fn land_and_no_other_card_type(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let kept = TypeSet::of(&[CardType::Land]);
    chars.types = kept;
    chars.subtypes = chars.subtypes.iter().filter(|s| s.card_types().0 & kept.0 != 0).fold(SubtypeSet::NONE, SubtypeSet::with);
}

pub static CARD: CardDef = CardDef {
    enchant: Some(target(creature_land_or_planeswalker)),
    statics: &[
        static_ability(Layer::Type, attached_object, land_and_no_other_card_type, "Enchanted permanent is a land and loses all other card types."),
        static_ability(Layer::Color, attached_object, colorless, "Enchanted permanent is colorless.").continues(),
        loses_all_abilities(attached_object, "Enchanted permanent loses all abilities.").continues(),
        // "... with '{T}: Add {C}'" (CR 613.1f): given after it lost the rest, so it keeps this one.
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Enchanted permanent has \"{T}: Add {C}.\""),
    ],
    delayed: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..oracle::IMPRISONED_IN_THE_MOON
};
