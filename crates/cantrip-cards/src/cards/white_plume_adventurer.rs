use cantrip_api::*;
use cantrip_mechanics::{choose, effects};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Untap a creature you control, chosen as it resolves and not targeted; if you've completed a dungeon, untap all
/// creatures you control instead (CR 309.7, 608.2d, 701.26). An already untapped creature may be chosen too.
fn untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let creatures: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = if cantrip_mechanics::history::completed_a_dungeon(g, you) {
        creatures
    } else {
        choose::card(g, you, item.object, Purpose::Untap, creatures)?.into_iter().collect()
    };
    g.untap_together(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::take_the_initiative, "When this creature enters, you take the initiative."),
        triggered(cantrip_mechanics::triggers::EACH_OPPONENTS_UPKEEP, untap, "At the beginning of each opponent's upkeep, untap a creature you control. If you've completed a dungeon, untap all creatures you control instead."),
    ],
    ..oracle::WHITE_PLUME_ADVENTURER
};
