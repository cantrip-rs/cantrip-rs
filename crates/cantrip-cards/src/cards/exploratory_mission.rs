use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Whenever a land you control enters, put a quest counter on this enchantment. Then exile cards from the top of your
/// library until you exile a nonland card. You may play that card this turn. If this enchantment has five or more quest
/// counters on it, add {R}{R}." The instructions resolve in order (CR 603.2, 608.2c); the hit is a nonland card, and only
/// that card gets this turn's play permission (CR 701.13a, 701.18b, 514.2). The threshold is checked after the counter is
/// added, including when it becomes the fifth quest counter.
fn landfall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Quest, 1);
    g.end_sentence();

    let (_, hit) = library::exile_until_nonland(g, item.controller, None, &|_, _| true);
    if let Some(card) = hit {
        g.allow_play(card, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    g.end_sentence();

    if g.obj(item.object).counters(CounterKind::Quest) >= 5 {
        g.add_mana(item.controller, Mana::R, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::LANDFALL,
        landfall,
        "Landfall — Whenever a land you control enters, put a quest counter on this enchantment. Then exile cards from the top of your library until you exile a nonland card. You may play that card this turn. If this enchantment has five or more quest counters on it, add {R}{R}.",
    )],
    ..oracle::EXPLORATORY_MISSION
};

