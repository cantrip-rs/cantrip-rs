use cantrip_api::*;
use cantrip_mechanics::{statics, tokens};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Create two 1/1 black Rat creature tokens with 'This token can't block.'" Both are made at once
/// (CR 701.7a); the Adventure's own exile as it resolves is the core's (CR 715.3d).
pub static PEST_PROBLEM: CardDef = CardDef { spell: spell(&[], tokens::create_rats_that_cant_block::<2>), ..oracle::PEST_PROBLEM };

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike.")],
    adventure: Some(&PEST_PROBLEM),
    ..oracle::RATCATCHER_TRAINEE
};

pub static EXTRA: &[&CardDef] = &[&PEST_PROBLEM, &tokens::RAT_CANT_BLOCK];
