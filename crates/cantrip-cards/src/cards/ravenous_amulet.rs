use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Draw a card and put a soul counter on this artifact": the controller draws, then the Amulet gets a counter; neither
/// instruction depends on the other, so the sentence is not split.
fn draw_and_soul_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.add_counters(item.object, CounterKind::Soul, 1);
    Ok(())
}

/// "Each opponent loses life equal to the number of soul counters on this artifact": the Amulet was sacrificed to pay for the
/// ability, so its counters are read from its last-known information (CR 608.2h), as Time Bomb reads its time counters.
fn drain_for_soul_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let souls = g.obj(item.object).counters(CounterKind::Soul).max(0);
    g.each_opponent_loses_life(item.controller, souls);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}, {T}").and(sacrifice(1, targets::filters::creature)), draw_and_soul_counter,
            "{1}, {T}, Sacrifice a creature: Draw a card and put a soul counter on this artifact. Activate only as a sorcery.")
            .sorcery_speed(),
        ability(cost("{4}, {T}").and(SACRIFICE_THIS), drain_for_soul_counters,
            "{4}, {T}, Sacrifice this artifact: Each opponent loses life equal to the number of soul counters on this artifact."),
    ],
    ..oracle::RAVENOUS_AMULET
};
