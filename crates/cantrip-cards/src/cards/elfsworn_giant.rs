use cantrip_api::*;
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Landfall — Whenever a land you control enters, create a 1/1 green Elf Warrior creature token." (CR 603.6a, 111.1, 701.7a).
/// Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        tokens::create_an_elf_warrior,
        "Landfall — Whenever a land you control enters, create a 1/1 green Elf Warrior creature token.",
    )],
    ..oracle::ELFSWORN_GIANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF_WARRIOR];
