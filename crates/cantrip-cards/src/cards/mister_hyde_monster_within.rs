use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_oracle as oracle;

/// "Remove a counter from a creature you control. If you do, draw a card." (CR 122.1, 608.2d, 608.2c): its controller chooses one
/// counter, of any kind, on any creature they control as the ability resolves - Mister Hyde himself included - and draws only if one
/// came off, so with no counter on any of their creatures there is no card.
fn remove_a_counter_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    if g.remove_a_counter_from_among(you, item.object, &creatures)? {
        g.end_sentence();
        g.draw(you, 1);
    }
    Ok(())
}

/// "At the beginning of your upkeep, choose one" (CR 603.3c, 700.2): the mode is chosen as the ability is put on the stack.
static MODAL: ModalDef = modes(&[
    mode(plus_one_counter_on_this, "Put a +1/+1 counter on Mister Hyde."),
    mode(remove_a_counter_and_draw, "Remove a counter from a creature you control. If you do, draw a card."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        YOUR_UPKEEP,
        &MODAL,
        "At the beginning of your upkeep, choose one \u{2014} Put a +1/+1 counter on Mister Hyde; or remove a counter from a creature you control. If you do, draw a card.",
    )],
    ..oracle::MISTER_HYDE_MONSTER_WITHIN
};
