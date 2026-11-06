use cantrip_api::*;
use cantrip_mechanics::effects::pump_all_creatures;
use cantrip_mechanics::greatest_power;
use cantrip_oracle as oracle;

/// "Draw cards equal to the greatest power among creatures you control. You lose life equal to the number of cards drawn
/// this way." The greatest power is read as the spell resolves (CR 608.2h), negative powers counting as nothing, and the
/// life lost is only for the cards actually drawn (CR 121.1), so a library with fewer cards draws fewer.
fn draw_for_greatest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let greatest = greatest_power::among_creatures_you_control(g, item.controller) as u32;
    let before = g.history().drawn(item.controller);
    g.draw(item.controller, greatest);
    let drawn = g.history().drawn(item.controller) - before;
    g.lose_life(item.controller,(drawn as i32) as i32);
    Ok(())
}

/// "All creatures get -3/-3 until end of turn": every creature on the battlefield as it resolves (CR 611.2c), whoever
/// controls it.
fn shrink_all(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump_all_creatures(g, item.object, -3, -3);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(draw_for_greatest, "Draw cards equal to the greatest power among creatures you control. You lose life equal to the number of cards drawn this way."),
    mode(shrink_all, "All creatures get -3/-3 until end of turn."),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::RISE_OF_THE_DEATHBRINGER
};
