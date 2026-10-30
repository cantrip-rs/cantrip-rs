use cantrip_api::*;
use cantrip_mechanics::library::{Rest, look_at_top};
use cantrip_mechanics::{creature_types_among_non_borg, triggers};
use cantrip_oracle as oracle;

/// "Federation — When Hoshi Sato enters, look at the top X cards of your library, where X is the number of creature types
/// among non-Borg creatures you control" (CR 205.3m, 603.6a): X is counted once as the ability resolves (CR 608.2h), the
/// creature types among every non-Borg creature its controller has then. One of the cards goes into that player's hand and
/// the rest to the bottom of their library in a random order; looking at no cards does nothing (CR 609.3).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let x = creature_types_among_non_borg(g, item.object, player);
    look_at_top(g, player, item.object, x, (1, 1), false, |_| true, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        look,
        "Federation \u{2014} When Hoshi Sato enters, look at the top X cards of your library, where X is the number of creature types among non-Borg creatures you control. Put one of them into your hand and the rest on the bottom of your library in a random order.",
    )],
    ..oracle::HOSHI_SATO_EXOLINGUIST
};
