use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers::YOU_DRAW_YOUR_SECOND_CARD};
use cantrip_oracle as oracle;

/// "Create a 1/1 colorless Spirit creature token with 'This token can't block or be blocked by non-Spirit creatures.'" (CR 111.1)
fn create_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        create_spirit,
        "Whenever you draw your second card each turn, create a 1/1 colorless Spirit creature token with \"This token can't block or be blocked by non-Spirit creatures.\"",
    )],
    ..oracle::FOGGY_SWAMP_SPIRIT_KEEPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT];
