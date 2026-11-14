use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Tap any number of untapped creatures you control. This creature gets +1/+1 until end of turn for each creature tapped this
/// way" (CR 701.26a, 611.2a): the creatures are chosen as the ability resolves, from the untapped creatures its controller
/// controls at that time (this creature too, if it has vigilance), and tapped together; the bonus counts those that were tapped.
fn tap_any_number_of_untapped_creatures_then_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let untapped: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.controller(id) == you && !g.state().obj(id).tapped && g.chars(id).is(CardType::Creature))
        .collect();
    let chosen = choose::select(g, you, item.object, Purpose::Tap, untapped.clone(), Selection::listed(select::ANY_NUMBER, &untapped, untapped.clone()))?;
    g.tap_together(&chosen);
    g.end_sentence();
    let tapped = chosen.iter().filter(|&&id| g.state().obj(id).tapped).count() as i32;
    g.add_effect(item.object, item.object, EffectKind::Pump(tapped, tapped), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        tap_any_number_of_untapped_creatures_then_grow,
        "Whenever this creature attacks, tap any number of untapped creatures you control. This creature gets +1/+1 until end of turn for each creature tapped this way.",
    )],
    ..oracle::ORPHANS_OF_THE_WHEAT
};
