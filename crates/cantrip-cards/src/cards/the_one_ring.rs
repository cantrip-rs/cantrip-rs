use cantrip_api::*;
use cantrip_oracle as oracle;

/// "You gain protection from everything until your next turn" (CR 702.16j, 611.2a).
fn protect_you(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::PlayerProtectionFromEverything, Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

/// "You lose 1 life for each burden counter on The One Ring", counted as the ability resolves; if it has left, as it
/// last was (CR 608.2h).
fn lose_life_for_burden(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let burdens = g.obj(item.object).counters(CounterKind::Burden);
    if burdens > 0 {
        g.lose_life(item.controller, burdens as i32);
    }
    Ok(())
}

/// "Put a burden counter on The One Ring, then draw a card for each burden counter on The One Ring."
fn add_burden_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Burden, 1);
    let burdens = g.obj(item.object).counters(CounterKind::Burden);
    if burdens > 0 {
        g.draw(item.controller, burdens as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST, protect_you, "When The One Ring enters, if you cast it, you gain protection from everything until your next turn."),
        triggered(cantrip_mechanics::triggers::YOUR_UPKEEP, lose_life_for_burden, "At the beginning of your upkeep, you lose 1 life for each burden counter on The One Ring."),
        ability(TAP, add_burden_and_draw, "{T}: Put a burden counter on The One Ring, then draw a card for each burden counter on The One Ring."),
    ],
    ..oracle::THE_ONE_RING
};
