use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RemoveACounter", "remove a +1/+1 counter from it?", "If you do, put a +1/+1 counter on each other creature you control", "Remove a counter", "Keep it");

/// "You may remove a +1/+1 counter from this creature. If you do, put a +1/+1 counter on each other creature you
/// control" (CR 122.1, 608.2c): the counter is removed as the ability resolves, and "if you do" asks whether one came
/// off. The creatures are read after the removal.
fn share(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = item.object;
    if !effects::ask_may(g, item.controller, this, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    if g.remove_counters(this, CounterKind::PlusOne, 1) == 0 {
        return Ok(());
    }
    effects::plus_one_counter_on_each_other_creature_you_control(g, item)
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[triggered(BEGINNING_OF_YOUR_COMBAT, share, "At the beginning of combat on your turn, you may remove a +1/+1 counter from this creature. If you do, put a +1/+1 counter on each other creature you control.")],
    ..oracle::GUIDING_HYDRA
};
