use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Draw", "pay {cost} to draw a card?", "", "Pay {cost}", "Don't pay");

/// "You may pay 1 life. If you do, draw a card." Only a player with the life is asked (CR 119.4).
fn pay_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(1), &ASK)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::SPELL_YOU_CAST_DURING_AN_OPPONENTS_TURN,
        pay_to_draw,
        "Whenever you cast a spell during an opponent's turn, you may pay 1 life. If you do, draw a card.",
    )],
    ..oracle::VORACIOUS_TOME_SKIMMER
};
