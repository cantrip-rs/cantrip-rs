use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, you may sacrifice an artifact. If you don't, tap this creature and it deals 2 damage to you." (CR 701.21a, 113.7a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::up_to(1)))?;
    g.end_sentence();
    if sacrificed.is_empty() {
        instruction(g, |g| {
            g.tap(item.object);
            g.damage(item.object, Target::Player(item.controller), 2);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            upkeep,
            "At the beginning of your upkeep, you may sacrifice an artifact. If you don't, tap this creature and it deals 2 damage to you.",
        ),
    ],
    ..oracle::YAWGMOTH_DEMON
};
