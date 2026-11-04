use cantrip_api::*;
use cantrip_mechanics::{effects, pump};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Each creature gets -X/-X until end of turn": every creature on the battlefield as this resolves, fixed then
/// (CR 611.2c), for the X it was cast with (entering without being cast, X is 0).
fn shrink_all(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.obj(item.object).x as i16;
    if x == 0 {
        return Ok(());
    }
    let creatures: Vec<ObjectId> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            pump(g, item.object, creature, -x, -x);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, shrink_all, "When The Meathook Massacre enters, each creature gets -X/-X until end of turn."),
        triggered(cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_DIES, cantrip_mechanics::effects::each_opponent_loses_1_life, "Whenever a creature you control dies, each opponent loses 1 life."),
        triggered(cantrip_mechanics::triggers::CREATURE_AN_OPPONENT_CONTROLS_DIES, |g, item| effects::gain_life(g, item, 1), "Whenever a creature an opponent controls dies, you gain 1 life."),
    ],
    ..oracle::THE_MEATHOOK_MASSACRE
};
