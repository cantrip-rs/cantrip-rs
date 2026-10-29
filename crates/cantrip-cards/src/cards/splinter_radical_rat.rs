use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "If a triggered ability of a Ninja creature you control triggers, that ability triggers an additional time." (CR 603.2d): the
/// creature as it is when the ability triggers, Splinter himself among them; the creature type is a word of the text (CR 612).
fn ninja_creature_you_control(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller) && {
        let c = q.chars(triggering.source);
        c.is(CardType::Creature) && c.has_subtype(q.words(asker.source).subtype(Subtype::Ninja))
    }
}

/// "Target Ninja": a Ninja on the battlefield, whoever controls it.
fn ninja(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield)
        && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Ninja)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{U}"), effects::target_creature_cant_be_blocked_this_turn, "{1}{U}: Target Ninja can't be blocked this turn.")
        .targets(&[target(ninja)])],
    additional_triggers: &[ninja_creature_you_control],
    ..oracle::SPLINTER_RADICAL_RAT
};
