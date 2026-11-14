use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{lands, statics, tokens};
use cantrip_oracle as oracle;

/// "When this creature enters, if you control another Elf, create a 1/1 green Elf Warrior creature token." (CR 603.4, 111.1, 701.7a):
/// an intervening "if", asked as the ability would trigger and again as it resolves. The Elf must be another permanent than this one,
/// and an opponent's Elf does not count; the token is an Elf itself, so a second Elite entering finds it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tokens::create_an_elf_warrior,
        "When this creature enters, if you control another Elf, create a 1/1 green Elf Warrior creature token.",
    )
    .condition(statics::you_control_another::<{ lands::sub(Subtype::Elf) }>)],
    ..oracle::DWYNENS_ELITE
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF_WARRIOR];
