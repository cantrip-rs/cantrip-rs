use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "Each opponent loses 5 life. You gain life equal to the life lost this way": measured rather than assumed, since an
/// effect can change what is lost (CR 119.3, 608.2c).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lost = g.each_opponent_loses_life(item.controller, 5);
    if lost > 0 {
        g.life_changes_together(&[LifeChange { player: item.controller, magnitude: lost, negative: false }]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, drain, "When Kokusho, the Evening Star dies, each opponent loses 5 life. You gain life equal to the life lost this way.")],
    ..oracle::KOKUSHO_THE_EVENING_STAR
};
