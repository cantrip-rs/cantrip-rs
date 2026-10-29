use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target attacking Goblin you control" (CR 115.1, 506.4): a creature of the Goblin type (a word of this card's text,
/// CR 612.2) its controller controls, attacking now.
fn attacking_goblin_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let goblin = q.words(asker.source).subtype(Subtype::Goblin);
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.is_attacking(id) && q.chars(id).has_subtype(goblin))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::target_creature_gets::<1, 0>,
        "Whenever you attack, target attacking Goblin you control gets +1/+0 until end of turn.",
    )
    .targets(&[target(attacking_goblin_you_control)])],
    ..oracle::BOGGART_PRANKSTER
};
