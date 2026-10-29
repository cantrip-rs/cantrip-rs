use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target Merfolk you control" (CR 115.1, 205.3m): a creature of the Merfolk type (a word of this card's text,
/// CR 612.2) its controller controls, other than this one.
fn another_merfolk_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let merfolk = q.words(asker.source).subtype(Subtype::Merfolk);
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if id != asker.source && q.chars(id).has_subtype(merfolk))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_BECOMES_TAPPED,
        effects::target_creature_gets::<2, 0>,
        "Whenever this creature becomes tapped, another target Merfolk you control gets +2/+0 until end of turn.",
    )
    .targets(&[target(another_merfolk_you_control)])],
    ..oracle::TRIBUTARY_VAULTER
};
