use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::{attacking_creatures, connive_n, triggers};
use cantrip_oracle as oracle;

/// The target connives X, where X is the number of attacking creatures, counted as the ability resolves over every
/// attacking creature, not only its controller's.
fn connive_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let attacking = u32::try_from(attacking_creatures(g)).unwrap_or(u32::MAX);
    // The target is still an attacking creature (CR 608.2b), so X is at least 1.
    connive_n(g, creature, item.object, attacking)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(triggers::YOU_ATTACK, connive_x, "Whenever you attack, target attacking creature connives X, where X is the number of attacking creatures.")
            .targets(&[targets::ATTACKING_CREATURE]),
    ],
    ..oracle::RAFFINE_SCHEMING_SEER
};
