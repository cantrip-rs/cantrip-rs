use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, tokens};
use cantrip_oracle as oracle;

/// "Choco-Comet deals X damage to any target. Create a 2/2 green Bird creature token with 'Whenever a land you control enters, this
/// token gets +1/+0 until end of turn.'" (CR 107.3f, 120.3, 111.1): X is the value announced as it was cast. With its one target
/// illegal as it tries to resolve, it does not resolve and no Bird is made (CR 608.2b; its ruling).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, item.x);
    tokens::create_a_landfall_bird(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    ..oracle::CHOCO_COMET
};

pub static EXTRA: &[&CardDef] = &[&tokens::LANDFALL_BIRD];
