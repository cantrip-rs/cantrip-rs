use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

const TEXT: &str = "Whenever this creature deals combat damage to a player, you may pay 1 life. If you do, draw a card.";
const ASK: Question = Question::new("Draw", "pay {cost} to draw a card?", "", "Pay {cost}", "Don't pay").no_default();

/// "You may pay 1 life. If you do, draw a card." (CR 118.12, 121.2).
fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(1), &ASK)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

/// "Artifact creatures you control have [the trigger]." (CR 613.1f, 603.3a).
pub static CARD: CardDef = CardDef {
    delayed: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, draw, TEXT)],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_artifact_creatures,
        statics::grant_delayed_ability,
        "Artifact creatures you control have \"Whenever this creature deals combat damage to a player, you may pay 1 life. If you do, draw a card.\"",
    )],
    ..oracle::HIVE_MIND_COPROCESSOR
};
