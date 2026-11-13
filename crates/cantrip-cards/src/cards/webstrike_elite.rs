use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target artifact or enchantment with mana value X" (CR 115.1, 202.3, 107.3a), X being the X of the cycling cost paid
/// (`triggers::YOU_CYCLE_THIS_CARD`, `Query::event_data`): chosen as the ability goes on the stack and judged again as it
/// resolves with the same X (CR 608.2b). With no X to read (the question of whether the ability could have a target) nothing
/// is, which "up to one" allows.
fn artifact_or_enchantment_with_mana_value_x(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment))
        && q.mana_value(id) == q.event_data()[0])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::cycling(mana("{X}{G}{G}"), "Cycling {X}{G}{G} ({X}{G}{G}, Discard this card: Draw a card.)"),
        triggered(triggers::YOU_CYCLE_THIS_CARD, effects::destroy_first_target,"When you cycle this card, destroy up to one target artifact or enchantment with mana value X.")
            .targets(&[up_to_one(artifact_or_enchantment_with_mana_value_x)]),
    ],
    ..oracle::WEBSTRIKE_ELITE
};
