use cantrip_api::*;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

const KINDS: [Subtype; 4] = [Subtype::Rabbit, Subtype::Bat, Subtype::Bird, Subtype::Mouse];

/// A Rabbit, Bat, Bird or Mouse (CR 205.3m): each type read through the words of this card's text (CR 612.2), so a text change
/// to one of them changes what counts.
fn is_a_rabbit_bat_bird_or_mouse(words: &Words, chars: &Chars) -> bool {
    KINDS.into_iter().any(|kind| chars.has_subtype(words.subtype(kind)))
}

/// "Whenever one or more other Rabbits, Bats, Birds, and/or Mice you control enter" (CR 603.6a): a permanent other than this one,
/// under your control as it entered, with one of the four types; `.once_per_batch()` makes the permanents entering together one
/// trigger, this one among them (the ruling).
fn another_rabbit_bat_bird_or_mouse_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source && to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && is_a_rabbit_bat_bird_or_mouse(&q.words(me.source), &q.chars(*new)) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Other Rabbits, Bats, Birds, and Mice you control" (CR 613.1f, layer 7c): every other permanent its controller controls with one
/// of the four types as the earlier layers made it.
fn other_rabbits_bats_birds_and_mice_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && q.controller(affected) == q.controller(source) && is_a_rabbit_bat_bird_or_mouse(&q.words(source), so_far)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_rabbit_bat_bird_or_mouse_you_control_enters),
        |g, item| effects::scry(g, item, 1),
        "Whenever one or more other Rabbits, Bats, Birds, and/or Mice you control enter, scry 1.",
    )
    .once_per_batch()],
    statics: &[static_ability(
        Layer::PtModify,
        other_rabbits_bats_birds_and_mice_you_control,
        statics::plus_pt::<1, 1>,
        "Other Rabbits, Bats, Birds, and Mice you control get +1/+1.",
    )],
    ..oracle::VALLEY_QUESTCALLER
};
