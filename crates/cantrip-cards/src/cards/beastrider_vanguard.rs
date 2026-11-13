use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Look at the top three cards of your library. You may reveal a permanent card from among them and put it into your hand. Put
/// the rest on the bottom of your library in any order." (CR 110.4a, 401.4, 701.20a)
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, 3, (0, 1), true, targets::is_permanent_card, Rest::BottomChosen)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{G}"),
        look,
        "{4}{G}: Look at the top three cards of your library. You may reveal a permanent card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
    )],
    ..oracle::BEASTRIDER_VANGUARD
};
