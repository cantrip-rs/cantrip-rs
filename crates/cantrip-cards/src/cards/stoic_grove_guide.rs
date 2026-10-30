use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 2/2 black and green Elf creature token Stoic Grove-Guide makes (CR 111.1, 205.4a).
pub static ELF: CardDef = CardDef {
    colors: Some(ColorSet((1 << Color::Black as u8) | (1 << Color::Green as u8))),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Elf)
};

/// "Create a 2/2 black and green Elf creature token."
fn make_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELF, 1)?;
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

pub static EXTRA: &[&CardDef] = &[&ELF];
