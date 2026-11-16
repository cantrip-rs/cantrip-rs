use cantrip_api::*;
use cantrip_mechanics::plot;
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it plus an additional +1/+1 counter on it for each other creature you control"
/// (CR 614.1c, 122.6): counted as it enters, among the creatures of the player it enters under. It is not on the battlefield yet,
/// so it never counts itself, and a creature entering with it is not counted either (its ruling).
fn enters_with_counters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let others = g.permanents_of(entering.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).count();
    entering.counters[CounterKind::PlusOne as usize] += 1 + others as i32;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters),
    abilities: &[plot::plot(mana("{1}{W}"), "Plot {1}{W}")],
    ..oracle::SHERIFF_OF_SAFE_PASSAGE
};
