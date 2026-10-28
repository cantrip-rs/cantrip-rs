use cantrip_api::*;
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/1 black Villain creature token with menace." (CR 111.1.)
fn villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::subtype_spell_you_cast::<{ lands::sub(Subtype::Villain) }>),
        villain,
        "Whenever you cast a Villain spell, create a 2/1 black Villain creature token with menace.",
    )],
    ..oracle::MADAME_HYDRA
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
