use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::{effects, triggers};

/// "Surveil 1. If her power is 4 or greater, instead draw a card." (CR 701.25, 613.1): test current power as this resolves.
fn tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.chars(item.object).power.unwrap_or(0) >= 4 {
        g.draw(item.controller, 1);
        Ok(())
    } else {
        effects::surveil(g, item, 1)
    }
}

/// "Whenever you face a dilemma, put a +1/+1 counter on Jett Reno." Dilemma choices are recorded in event history.
fn add_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_BECOMES_TAPPED, tapped, "Whenever Jett Reno becomes tapped, surveil 1. If her power is 4 or greater, instead draw a card."),
        triggered(triggers::YOU_FACE_A_DILEMMA, add_counter, "Whenever you face a dilemma, put a +1/+1 counter on Jett Reno."),
    ],
    ..oracle::JETT_RENO_PROBLEM_SOLVER
};
