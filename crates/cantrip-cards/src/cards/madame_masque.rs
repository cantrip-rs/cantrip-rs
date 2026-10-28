use cantrip_api::*;
use cantrip_mechanics::{connive, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Madame Masque enters, she connives." (CR 701.50): she connives even if she has already left the battlefield.
fn connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    connive(g, item.object, item.object)
}

/// "Create a 2/1 black Villain creature token with menace." (CR 111.1.)
fn villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, connives, "When Madame Masque enters, she connives."),
        triggered(
            triggers::YOU_DRAW_YOUR_SECOND_CARD,
            villain,
            "Whenever you draw your second card each turn, create a 2/1 black Villain creature token with menace.",
        ),
    ],
    ..oracle::MADAME_MASQUE
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
