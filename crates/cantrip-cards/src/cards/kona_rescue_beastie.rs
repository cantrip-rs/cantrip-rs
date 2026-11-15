use cantrip_api::*;
use cantrip_mechanics::{hand, targets, triggers};
use cantrip_oracle as oracle;

/// "You may put a permanent card from your hand onto the battlefield." Up to one permanent card (CR 110.4a); nothing is
/// asked when the hand has none. An Aura chooses what it enchants as it enters (CR 303.4f).
fn put_permanent_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_onto_battlefield(g, item.controller, item.object, targets::is_permanent_card)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        put_permanent_card,
        "Survival \u{2014} At the beginning of your second main phase, if Kona is tapped, you may put a permanent card from your hand onto the battlefield.",
    )],
    ..oracle::KONA_RESCUE_BEASTIE
};
