use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create two 1/1 white Soldier creature tokens. Until end of turn, creatures you control get +1/+1 and gain haste." (CR 608.2c,
/// 611.2c): the tokens are made first, so they are among the creatures that get the bonus - the set is fixed as the second
/// sentence resolves, and a creature that enters later this turn gets nothing.
fn reinforce(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SOLDIER, 2)?;
    g.end_sentence();
    effects::creatures_you_control_get_and_gain::<1, 1, { Keywords::of(&[Keyword::Haste]).0 }>(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], reinforce), ..oracle::HEROIC_REINFORCEMENTS };

pub static EXTRA: &[&CardDef] = &[&tokens::SOLDIER];
