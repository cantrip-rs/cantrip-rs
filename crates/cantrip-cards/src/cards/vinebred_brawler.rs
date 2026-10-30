use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This creature must be blocked if able" (CR 509.1c): the requirement is on this creature as an attacker, and any creature that
/// can block it obeys it. A creature that can't be blocked, or that nothing able to block can reach, is required nothing (the rulings).
fn this_creature_must_be_blocked(_: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    attacker == source
}

/// "Another target Elf you control" (CR 115.1, 109.5): a creature of the Elf type (a word of this card's text, CR 612.2) its
/// controller controls, other than this one.
fn another_elf_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let elf = q.words(asker.source).subtype(Subtype::Elf);
    (targets::OTHER_CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.chars(id).has_subtype(elf))
}

pub static CARD: CardDef = CardDef {
    must_be_blocked: &[this_creature_must_be_blocked],
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::target_creature_gets::<2, 1>,
        "Whenever this creature attacks, another target Elf you control gets +2/+1 until end of turn.",
    )
    .targets(&[target(another_elf_you_control)])],
    ..oracle::VINEBRED_BRAWLER
};
