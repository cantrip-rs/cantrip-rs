use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature or another creature you control with flying dies" (CR 700.4, 603.10a): this creature, or a creature
/// that had flying as it last was on the battlefield under the controller it last had; the trigger is about that creature as it
/// last existed (its mana value, `StackItem::subject`).
fn this_or_another_flying_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::creature_you_control_dies(q, me, event)?;
    match event {
        Event::ZoneChange { old, .. } if *old == me.source || q.chars(*old).has(Keyword::Flying) => Fired::about(Target::Object(*old)),
        _ => None,
    }
}

/// "Another target creature card with lesser mana value from your graveyard" (CR 115.1, 202.3): a creature card in your
/// graveyard whose mana value is less than that of the creature that died, as it last existed. A card cannot have a lesser mana
/// value than itself, so "another" excludes the dying creature's card without a check of its own.
fn creature_card_with_lesser_mana_value_in_your_graveyard(q: &dyn Query, me: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, me, t, &[CardType::Creature])
        && matches!((t, q.event_subject()), (Target::Object(id), Target::Object(dead)) if q.mana_value(id) < q.mana_value(dead))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_flying_creature_you_control_dies),
        effects::return_first_target_to_battlefield,
        "Whenever this creature or another creature you control with flying dies, return another target creature card with lesser mana value from your graveyard to the battlefield.",
    )
    .targets(&[target(creature_card_with_lesser_mana_value_in_your_graveyard)])],
    ..oracle::JACKDAW_SAVIOR
};
