use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;


/// "Create a 2/1 black Villain creature token with menace." (CR 111.1.)
fn villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::this_connives, "When Madame Masque enters, she connives."),
        triggered(
            triggers::YOU_DRAW_YOUR_SECOND_CARD,
            villain,
            "Whenever you draw your second card each turn, create a 2/1 black Villain creature token with menace.",
        ),
    ],
    ..oracle::MADAME_MASQUE
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
