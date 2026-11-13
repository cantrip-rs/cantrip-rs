use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "Each opponent loses 5 life. You gain life equal to the life lost this way."
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_loses_life_you_gain_life_lost(g, item.controller, 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, drain, "When Kokusho, the Evening Star dies, each opponent loses 5 life. You gain life equal to the life lost this way.")],
    ..oracle::KOKUSHO_THE_EVENING_STAR
};
