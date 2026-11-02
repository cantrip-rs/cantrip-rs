use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create two 1/1 black and green Pest creature tokens with "When this token dies, you gain 1 life."" (CR 608.2, 111.1).
fn create_two_1_1_black(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::PEST_1_1_BLACK_GREEN);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(2));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], create_two_1_1_black),
    ..oracle::PEST_SUMMONING
};

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_1_1_BLACK_GREEN];
