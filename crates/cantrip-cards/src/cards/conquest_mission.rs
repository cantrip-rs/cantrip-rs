use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever a nontoken artifact you control enters, put a quest counter on this enchantment. Create a 1/1 colorless
/// Borg artifact creature token. If this enchantment has six or more quest counters on it, create two of those tokens
/// instead." The counter is added before checking the threshold; token creation is one event (CR 603.6a, 122.1,
/// 701.7a, 608.2c).
fn quest_and_borg(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Quest, 1);
    let count = if g.obj(item.object).counters(CounterKind::Quest) >= 6 { 2 } else { 1 };
    tokens::create(g, item.controller, &tokens::BORG, count)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONTOKEN_ARTIFACT_YOU_CONTROL_ENTERS,
        quest_and_borg,
        "Whenever a nontoken artifact you control enters, put a quest counter on this enchantment. Create a 1/1 colorless Borg artifact creature token. If this enchantment has six or more quest counters on it, create two of those tokens instead.",
    )],
    ..oracle::CONQUEST_MISSION
};

pub static EXTRA: &[&CardDef] = &[&tokens::BORG];
