use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, add {R}. Until end of turn, you don't lose this mana as steps and phases end" (CR 106.3,
/// 106.4, 514.2): the red mana is added until end of turn rather than lost as the steps before the turn's end pass.
fn add_r_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana_until_end_of_turn(item.controller, Mana::R, 1);
    Ok(())
}

// First strike is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        add_r_until_end_of_turn,
        "Whenever this creature attacks, add {R}. Until end of turn, you don't lose this mana as steps and phases end.",
    )],
    ..oracle::BRAZEN_COLLECTOR
};
