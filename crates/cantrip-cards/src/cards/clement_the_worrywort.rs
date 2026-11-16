use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control with lesser mana value" (CR 115.1, 202.3): a creature its controller controls whose mana value is
/// less than that of the creature that entered, the trigger's subject. Judged as the target is chosen and again as the ability
/// resolves (CR 608.2b).
fn creature_you_control_with_lesser_mana_value(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let (Target::Object(id), Target::Object(entered)) = (t, q.event_subject()) else { return false };
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && q.mana_value(id) < q.mana_value(entered)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_ENTERS,
        effects::return_first_target_to_hand,
        "Whenever Clement or another creature you control enters, return up to one target creature you control with lesser mana value to its owner's hand.",
    )
    .targets(&[target(creature_you_control_with_lesser_mana_value).optional()])],
    delayed: &[mana_ability(
        TAP,
        add_one_of(&[Color::Green, Color::Blue]).only_for(ManaRestriction::CREATURE_SPELLS),
        "{T}: Add {G} or {U}. Spend this mana only to cast a creature spell.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_of_subtype::<{ lands::sub(Subtype::Frog) }>,
        statics::grant_delayed_ability,
        "Frogs you control have \"{T}: Add {G} or {U}. Spend this mana only to cast a creature spell.\"",
    )],
    ..oracle::CLEMENT_THE_WORRYWORT
};
