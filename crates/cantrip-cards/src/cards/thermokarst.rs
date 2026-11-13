use cantrip_api::*;
use cantrip_mechanics::{destroy_target, targets};
use cantrip_oracle as oracle;

/// "Destroy target land. If that land was a snow land, you gain 1 life." (CR 608.2h, 701.8a, 119.3).
fn destroy_target_land_if_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = destroy_target(g, item, 0) else { return Ok(()) };
    g.end_sentence();
    if g.chars(land).supertypes.snow && g.chars(land).is(CardType::Land) {
        g.gain_life(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::LAND], destroy_target_land_if_land),
    ..oracle::THERMOKARST
};
