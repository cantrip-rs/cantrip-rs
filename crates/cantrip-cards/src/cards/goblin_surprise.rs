use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::creatures_you_control_get::<2, 0>, "Creatures you control get +2/+0 until end of turn."),
    mode(tokens::create_goblins::<2>, "Create two 1/1 red Goblin creature tokens."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GOBLIN_SURPRISE };

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
