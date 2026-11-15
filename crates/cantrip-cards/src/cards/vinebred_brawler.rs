use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{combat, effects, lands, targets};
use cantrip_oracle as oracle;

/// "Another target Elf you control" (CR 115.1, 109.2, 205.3m): the text names no card type, so a permanent with the Elf type
/// (a word of this card's text, CR 612.2) its controller controls is a legal target, a noncreature one included, other than this one.
fn another_elf_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    t != Target::Object(asker.source) && targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>(q, asker, t)
}

pub static CARD: CardDef = CardDef {
    must_be_blocked: &[combat::this_must_be_blocked_if_able],
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::target_creature_gets::<2, 1>,
        "Whenever this creature attacks, another target Elf you control gets +2/+1 until end of turn.",
    )
    .targets(&[target(another_elf_you_control)])],
    ..oracle::VINEBRED_BRAWLER
};
