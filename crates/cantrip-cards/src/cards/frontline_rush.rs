use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gets +X/+X until end of turn, where X is the number of creatures you control." X is worked out once,
/// as this resolves (official ruling); with the target illegal nothing happens (CR 608.2b).
fn rush(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let x = creatures_you_control(g, item.controller) as i32;
        pump(g, item.object, creature, x, x);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(tokens::create_goblins::<2>, "Create two 1/1 red Goblin creature tokens."),
    mode(rush, "Target creature gets +X/+X until end of turn, where X is the number of creatures you control.")
        .targets(&[targets::CREATURE]),
])
.choose(1, 1);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FRONTLINE_RUSH };

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
