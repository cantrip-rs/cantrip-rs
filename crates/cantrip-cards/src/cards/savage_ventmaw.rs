use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Add {R}{R}{R}{G}{G}{G}. Until end of turn, you don't lose this mana as steps and phases end" (CR 106.4, 500.5): three red
/// and three green, added until end of turn rather than lost as the steps and phases before the turn's end pass.
fn add_three_red_and_three_green_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana_until_end_of_turn(item.controller, Mana::R, 3);
    g.add_mana_until_end_of_turn(item.controller, Mana::G, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        add_three_red_and_three_green_until_end_of_turn,
        "Whenever this creature attacks, add {R}{R}{R}{G}{G}{G}. Until end of turn, you don't lose this mana as steps and phases end.",
    )
    .produces_mana()],
    ..oracle::SAVAGE_VENTMAW
};
