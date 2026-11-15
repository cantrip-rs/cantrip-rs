use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::ATTACHED_CREATURE_ATTACKS_SUBJECT;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "You may sacrifice a permanent other than that creature or this Equipment. If you do, draw a card." (CR 701.21a, 608.2c-d):
/// the choice is made as the ability resolves, among the controller's other permanents (the attacking creature is the trigger's
/// subject, this Equipment its source); declining is the empty answer, and only a sacrifice that happened draws.
fn sacrifice_another_permanent_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let eligible: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&p| p != item.object && item.subject.object() != Some(p)).collect();
    let chosen = choose::select(g, you, item.object, Purpose::Sacrifice, eligible, select::cards(select::up_to(1)))?;
    if g.sacrifice(you, &chosen).is_empty() {
        return Ok(());
    }
    g.draw(you, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            ATTACHED_CREATURE_ATTACKS_SUBJECT,
            sacrifice_another_permanent_to_draw,
            "Whenever equipped creature attacks, you may sacrifice a permanent other than that creature or this Equipment. If you do, draw a card.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    ..oracle::SAW
};
