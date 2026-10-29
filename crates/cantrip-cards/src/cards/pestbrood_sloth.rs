use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this creature dies, create two 1/1 black and green Pest creature tokens with 'Whenever this token attacks, you gain 1 life.'"
fn pests(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        pests,
        "When this creature dies, create two 1/1 black and green Pest creature tokens with \"Whenever this token attacks, you gain 1 life.\"",
    )],
    ..oracle::PESTBROOD_SLOTH
};

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK];
