use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The legal target gains first strike until end of turn (CR 608.2b, 702.7b).
fn first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = targets::object(g, item, 0) {
        effects::gain_until_end_of_turn(g, item.object, target, Keyword::FirstStrike);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(TAP),
        first_strike,
        "{3}, {T}: Target creature gains first strike until end of turn.",
    ).targets(&[targets::CREATURE])],
    ..oracle::FYNDHORN_BOW
};
