use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{fight, lands, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a Monster Role token attached to it" (CR 111.10k, 303.7): the Role is attached by the effect, not targeted (the Role
/// rulings); with the Troll no longer on the battlefield by then, or no longer a creature, the Role is not created (CR 303.4i).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_role(g, &tokens::MONSTER_ROLE, item.controller, item.object)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_role, "When this creature enters, create a Monster Role token attached to it."),
        ability(
            cost("{1}").and(Cost::sacrifice(1, filters::subtype_attached_to_this::<{ lands::sub(Subtype::Aura) }>)),
            fight::this_fights_target_exiling_it_if_it_would_die,
            "{1}, Sacrifice an Aura attached to this creature: This creature fights target creature you don't control. If that creature would die this turn, exile it instead. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE_YOU_DONT_CONTROL])
        .sorcery_speed(),
    ],
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::FAUNSBANE_TROLL
};

pub static EXTRA: &[&CardDef] = &[&tokens::MONSTER_ROLE];
