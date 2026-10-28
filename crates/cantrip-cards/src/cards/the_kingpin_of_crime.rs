use cantrip_api::*;
use cantrip_mechanics::extort::EXTORT;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_oracle as oracle;

const PAY_LIFE: Question = Question::new(
    "ToughnessDamage",
    "pay {cost} so your creatures assign combat damage by toughness?",
    "Until end of turn, creatures you control with toughness greater than their power assign combat damage equal to their toughness",
    "Pay {cost}",
    "Don't pay",
);

/// "Whenever you attack, you may pay 2 life. If you do, until end of turn, creatures you control with toughness greater than
/// their power assign combat damage equal to their toughness rather than their power" (CR 119.4, 510.1a). A player with less
/// than 2 life can't pay it and `may_pay` does not ask them.
fn attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if g.may_pay(you, item.object, item.object, life(2), &PAY_LIFE)? {
        g.add_player_effect(you, item.object, EffectKind::CreaturesAssignDamageByToughness, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        EXTORT,
        triggered(
            YOU_ATTACK,
            attack,
            "Whenever you attack, you may pay 2 life. If you do, until end of turn, creatures you control with toughness greater than their power assign combat damage equal to their toughness rather than their power.",
        ),
    ],
    ..oracle::THE_KINGPIN_OF_CRIME
};
