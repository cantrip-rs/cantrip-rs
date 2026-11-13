use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this creature dies, draw a card for each +1/+1 counter on it" (CR 603.10a, 608.2h): the counters it had as it last
/// existed on the battlefield, all the cards drawn together as one draw. None for a creature that had none.
fn draw_a_card_for_each_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counters = g.obj(item.object).counters(CounterKind::PlusOne);
    if counters > 0 {
        g.draw(item.controller, counters as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[
        ability(cost("{4}"), effects::plus_one_counter_on_this, "{4}: Put a +1/+1 counter on this creature."),
        triggered(THIS_DIES, draw_a_card_for_each_counter, "When this creature dies, draw a card for each +1/+1 counter on it."),
    ],
    ..oracle::MARKETBACK_WALKER
};
