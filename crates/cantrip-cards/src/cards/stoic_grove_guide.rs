use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create a 2/2 black and green Elf creature token."
fn make_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ELF_BLACK_GREEN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B/G}").and(EXILE_THIS),
        make_elf,
        "{1}{B/G}, Exile this card from your graveyard: Create a 2/2 black and green Elf creature token. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .from(ZoneKind::Graveyard)],
    ..oracle::STOIC_GROVE_GUIDE
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELF_BLACK_GREEN];
