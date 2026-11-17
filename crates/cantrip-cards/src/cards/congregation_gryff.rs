use cantrip_api::*;
use cantrip_mechanics::{permanents_with_subtype_you_control, pump, saddle, triggers};
use cantrip_oracle as oracle;

/// "It gets +X/+X until end of turn, where X is the number of Mounts you control" (CR 611.2a, 608.2h, 205.3): X is counted once,
/// as the ability resolves, this creature among the Mounts while it is one, the subtype read through the words of this card's
/// text (CR 612).
fn pump_by_mounts(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mount = g.words(item.object).subtype(Subtype::Mount);
    let x = permanents_with_subtype_you_control(g, item.controller, mount) as i32;
    pump(g, item.object, item.object, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS_WHILE_SADDLED,
            pump_by_mounts,
            "Whenever this creature attacks while saddled, it gets +X/+X until end of turn, where X is the number of Mounts you control.",
        ),
        saddle::ability(3, "Saddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::CONGREGATION_GRYFF
};
