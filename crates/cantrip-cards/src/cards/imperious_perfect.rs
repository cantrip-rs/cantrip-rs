use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens};
use cantrip_oracle as oracle;

/// "Other Elves you control get +1/+1." (CR 613.4c, 205.3m): the Elf token the second ability makes is an Elf too, so it gets the bonus
/// as well; another player's Elves and Imperious Perfect itself do not.
/// "{G}, {T}: Create a 1/1 green Elf Warrior creature token." (CR 111.1, 701.7a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{G}").and(TAP),
        tokens::create_an_elf_warrior,
        "{G}, {T}: Create a 1/1 green Elf Warrior creature token.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        statics::plus_pt::<1, 1>,
        "Other Elves you control get +1/+1.",
    )],
    ..oracle::IMPERIOUS_PERFECT
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF_WARRIOR];
