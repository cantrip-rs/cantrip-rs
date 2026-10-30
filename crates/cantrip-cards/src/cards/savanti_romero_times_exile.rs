use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on Savanti Romero. Then you draw X cards and lose X life, where X is the number of counters on Savanti
/// Romero" (CR 122.1, 107.3): counters of every kind, counted once as this resolves (after the new one is on it) and used for
/// both. The counters it last had, if it has left the battlefield (CR 608.2h).
fn grow_then_draw_and_lose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    let x = effects::counters_on(g, item.object);
    g.draw(item.controller, x);
    g.lose_life(item.controller, x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        grow_then_draw_and_lose,
        "At the beginning of combat on your turn, put a +1/+1 counter on Savanti Romero. Then you draw X cards and lose X life, where X is the number of counters on Savanti Romero.",
    )],
    ..oracle::SAVANTI_ROMERO_TIMES_EXILE
};
