use cantrip_api::*;
use cantrip_mechanics::combat::create_attacking_of_your_choice;
use cantrip_mechanics::tokens::RABBIT;
use cantrip_mechanics::{effects, offspring, triggers};
use cantrip_oracle as oracle;

/// Mode 1: a 1/1 white Rabbit token, tapped and attacking (CR 508.4): its controller chooses what it attacks as it enters, a
/// player, planeswalker or battle, asked only when there is a choice. It was never declared as an attacker, so this ability does
/// not trigger again for it (the ruling).
fn rabbit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rabbit = g.card_id(&RABBIT);
    create_attacking_of_your_choice(g, item.controller, item.object, rabbit, 1, true)?;
    Ok(())
}

/// Mode 2: "Attacking creatures you control get +1/+1 until end of turn" (CR 611.2c): the creatures attacking as it resolves
/// that its controller controls, and none that attack later.
fn attackers_get(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    effects::pump_creatures_where(g, item.object, |q, id| q.is_attacking(id) && q.controller(id) == you, 1, 1);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(rabbit, "Create a 1/1 white Rabbit creature token that's tapped and attacking."),
    mode(attackers_get, "Attacking creatures you control get +1/+1 until end of turn."),
]);

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        modal_triggered(triggers::YOU_ATTACK, &MODAL, "Whenever you attack, choose one \u{2014} Create a 1/1 white Rabbit creature token that's tapped and attacking.; or Attacking creatures you control get +1/+1 until end of turn."),
    ],
    ..oracle::WARREN_WARLEADER
};

pub static EXTRA: &[&CardDef] = &[&RABBIT];
