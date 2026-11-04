use cantrip_api::*;
use cantrip_mechanics::effects::{destroy_first_target, this_gains_indestructible_then_tap};
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{history, tokens};
use cantrip_oracle as oracle;

/// "If a creature died under an opponent's control this turn" (CR 603.4, 700.4): asked of the history as the step begins, when it
/// would trigger, and again as it resolves. A token counts, and so does a creature of any opponent.
fn a_creature_died_under_an_opponents_control_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.state().opponents(asker.controller).any(|opponent| history::creatures_died_this_turn(q, Some(opponent)) > 0)
}

/// "Create a Treasure token. Then if you control three or more Treasures, transform this enchantment." (CR 608.2c, 701.27a): the
/// Treasure is made first, so it counts itself; the transformation does nothing if this permanent has left or already
/// transformed since the ability went on the stack (CR 701.27f).
fn create_a_treasure_then_maybe_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)?;
    if tokens::treasures_controlled(g, item.controller).len() >= 3 {
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B}").and(sacrifice(1, filters::another_creature_or_artifact)),
        this_gains_indestructible_then_tap,
        "{1}{B}, Sacrifice another creature or artifact: Yiazmat gains indestructible until end of turn. Tap it.",
    )],
    ..oracle::YIAZMAT_ULTIMATE_MARK
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, destroy_first_target, "When this enchantment enters, destroy up to one target creature.")
            .targets(&[targets::CREATURE.optional()]),
        triggered(
            YOUR_END_STEP,
            create_a_treasure_then_maybe_transform,
            "At the beginning of your end step, if a creature died under an opponent's control this turn, create a Treasure token. Then if you control three or more Treasures, transform this enchantment.",
        )
        .condition(a_creature_died_under_an_opponents_control_this_turn),
    ],
    back: Some(&BACK),
    ..oracle::SIDEQUEST_HUNT_THE_MARK
};
