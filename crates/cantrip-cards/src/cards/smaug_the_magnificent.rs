use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{create_a_treasure, treasures_controlled, TREASURE};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "He deals damage equal to the number of Treasures you control to any target": counted as the ability resolves, by
/// the ability's controller (CR 608.2h); no Treasures is no damage (CR 120.8), and a target that has since become
/// illegal is left alone (CR 608.2b).
fn treasure_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let treasures = treasures_controlled(g, item.controller).len();
    if treasures > 0 && g.target_legal(item, 0) {
        g.damage(item.object, item.target(0), i64::from(treasures as u16));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, treasure_damage, "Whenever Smaug attacks, he deals damage equal to the number of Treasures you control to any target.")
            .targets(&[targets::ANY]),
        triggered(triggers::YOUR_UPKEEP, create_a_treasure, "At the beginning of your upkeep, create a Treasure token."),
    ],
    ..oracle::SMAUG_THE_MAGNIFICENT
};

pub static EXTRA: &[&CardDef] = &[&TREASURE];
