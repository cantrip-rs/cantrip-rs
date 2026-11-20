use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Any number of target enchanted creatures you control" (CR 601.2c, 303.4b): a creature its caster controls with an Aura attached to it,
/// whoever controls the Aura.
fn enchanted_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if targets::is_enchanted(q, id))
}

/// "Up to one other target creature you control" (CR 115.1, 115.3): a creature its caster controls that is not one already chosen as an
/// enchanted creature for the same spell.
fn other_creature_you_control_than_those_chosen(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t) && !q.chosen_targets().contains(&t)
}

/// "... each deal damage equal to their power to target creature you don't control" (CR 120.3, 120.4b, 608.2b): the targets before
/// the last are the sources, the last the recipient (`fight::targets_each_bite_last`).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_each_bite_last(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            target(enchanted_creature_you_control).between(0, TargetSpec::ANY_NUMBER),
            target(other_creature_you_control_than_those_chosen).optional(),
            targets::CREATURE_YOU_DONT_CONTROL,
        ],
        deal_damage,
    ),
    ..oracle::GRACEFUL_TAKEDOWN
};
