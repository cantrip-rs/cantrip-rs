use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The 1/1 colorless Shapeshifter creature token with changeling.
pub static SHAPESHIFTER: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Changeling]),
    text: "Changeling",
    ..token!(Shapeshifter)
};

/// "Exile target creature. Its controller creates a 1/1 colorless Shapeshifter creature token with changeling": the controller
/// it had on the battlefield (CR 608.2h), who makes the token even though the creature is gone. A target that has become illegal
/// leaves the spell not resolving, so no token (ruling).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(creature);
    g.exile(creature);
    tokens::create(g, controller, &SHAPESHIFTER, 1)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&SHAPESHIFTER];

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::CRIB_SWAP };
