use cantrip_api::*;
use cantrip_mechanics::{effects, attach::attached_object, lands, statics::{artifact_of_subtype_alone, colorless, grant_delayed_ability}, targets};
use cantrip_oracle as oracle;

const ENCHANTED: &str = "Enchanted permanent is a colorless Food artifact with \"{2}, {T}, Sacrifice this artifact: You gain 3 life\" and loses all other card types and abilities.";

/// "Enchant creature or Food" (CR 303.4, 205.3g): a creature, or a permanent with the artifact type Food as the word is on this
/// card's text now (CR 612.2). A creature that has become a Food stays a legal thing to enchant, but only through being a Food.
fn creature_or_food(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && (q.chars(id).is(CardType::Creature) || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Food))))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(target(creature_or_food)),
    statics: &[
        static_ability(Layer::Type, attached_object, artifact_of_subtype_alone::<{ lands::sub(Subtype::Food) }>, ENCHANTED),
        static_ability(Layer::Color, attached_object, colorless, ENCHANTED).continues(),
        loses_all_abilities(attached_object, ENCHANTED).continues(),
        // "... with '{2}, {T}, Sacrifice this artifact: You gain 3 life'" (CR 613.1f): given after it lost the rest, so it keeps this one.
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, ENCHANTED),
    ],
    delayed: &[ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this artifact: You gain 3 life")],
    ..oracle::SUGAR_COAT
};
