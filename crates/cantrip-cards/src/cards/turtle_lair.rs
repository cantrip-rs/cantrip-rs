use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target Ninja or Turtle" (CR 115.1, 205.3): a permanent on the battlefield with either subtype, a creature in effect.
/// The words are the land's own (CR 612).
fn ninja_or_turtle(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if {
            let chars = q.chars(id);
            chars.has_subtype(q.words(asker.source).subtype(Subtype::Ninja)) || chars.has_subtype(q.words(asker.source).subtype(Subtype::Turtle))
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        // "Spend this mana only to cast a Ninja or Turtle spell" (CR 106.6, 205.3): a spell with either subtype as cast.
        mana_ability(
            TAP,
            add_any_color(1).only_for(ManaRestriction::Subtypes(SubtypeSpells::either(Subtype::Ninja, Subtype::Turtle))),
            "{T}: Add one mana of any color. Spend this mana only to cast a Ninja or Turtle spell.",
        ),
        ability(cost("{3}, {T}"), effects::target_creature_cant_be_blocked_this_turn, "{3}, {T}: Target Ninja or Turtle can't be blocked this turn.")
            .targets(&[target(ninja_or_turtle)]),
    ],
    ..oracle::TURTLE_LAIR
};
