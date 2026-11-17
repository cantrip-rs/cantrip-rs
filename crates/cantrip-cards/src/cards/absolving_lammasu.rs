use cantrip_api::*;
use cantrip_mechanics::effects::gain_life;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "All suspected creatures are no longer suspected" (CR 701.60a): every creature on the battlefield with the designation,
/// whoever controls it, as the ability resolves. A suspected permanent that is no creature is not one of them.
fn unsuspect_all_creatures(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let suspected: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature) && g.is_suspected(id)).collect();
    // One instruction, whatever the number of creatures (CR 608.2c).
    instruction(g, |g| {
        for creature in suspected {
            g.unsuspect(creature);
        }
    });
    Ok(())
}

/// "You gain 3 life and suspect up to one target creature an opponent controls" (CR 119.3, 701.60a): the life is gained even
/// with no target chosen, and a chosen target that is no longer legal as this resolves is not suspected (CR 608.2b).
fn gain_three_and_suspect_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    gain_life(g, item, 3)?;
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.suspect(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, unsuspect_all_creatures, "When this creature enters, all suspected creatures are no longer suspected."),
        triggered(
            triggers::THIS_DIES,
            gain_three_and_suspect_target,
            "When this creature dies, you gain 3 life and suspect up to one target creature an opponent controls. (A suspected creature has menace and can't block.)",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.between(0, 1)]),
    ],
    ..oracle::ABSOLVING_LAMMASU
};
