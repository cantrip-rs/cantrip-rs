use cantrip_api::*;
use cantrip_mechanics::typecycling;
use cantrip_oracle as oracle;

/// "Boars you control can't be blocked by more than one creature." (CR 509.1b, 113.1a): asked of each attacker with Rocksteady as
/// the source; one that is a Boar (read through the words of Rocksteady's text, CR 612.2) under its controller's control is
/// limited to one blocker, as long as Rocksteady is on the battlefield with its abilities. Rocksteady is not a Boar, and has
/// its own limit as the keyword.
fn boars_one_blocker_at_most(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> Option<usize> {
    let boar = q.words(source).subtype(Subtype::Boar);
    (q.controller(attacker) == q.controller(source) && q.chars(attacker).has_subtype(boar)).then_some(1)
}

pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBeBlockedByMoreThanOne]),
    abilities: &[typecycling::ability(Subtype::Forest, mana("{2}"), "Forestcycling {2}")],
    blocker_limits: &[boars_one_blocker_at_most],
    ..oracle::ROCKSTEADY_CRASH_COURSER
};
