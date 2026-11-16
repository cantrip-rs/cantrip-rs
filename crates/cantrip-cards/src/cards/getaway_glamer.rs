use cantrip_api::*;
use cantrip_mechanics::{delayed, destroy_target, spree, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature if no other creature has greater power." (CR 608.2b, 208.1): the comparison is made as the mode
/// resolves, after any earlier mode has been followed (so a creature the first mode exiled no longer counts), against the
/// power every other creature on the battlefield has now. A target that is no longer legal is not destroyed, and is not
/// compared.
fn destroy_if_no_other_creature_has_greater_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(power) = g.chars(target).power else { return Ok(()) };
    // A permanent that is not a creature has no power (CR 208.3), so an uncrewed Vehicle does not outpower the target.
    let outpowered = g.state().battlefield().iter().copied().filter(|&id| id != target).any(|id| g.chars(id).power.is_some_and(|p| p > power));
    if !outpowered {
        destroy_target(g, item, 0);
    }
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(
        delayed::exile_target_then_return_at_next_end_step_0,
        "+ {1} \u{2014} Exile target nontoken creature. Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )
    .targets(&[targets::NONTOKEN_CREATURE])
    .cost(mana("{1}")),
    mode(destroy_if_no_other_creature_has_greater_power, "+ {2} \u{2014} Destroy target creature if no other creature has greater power.")
        .targets(&[targets::CREATURE])
        .cost(mana("{2}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::GETAWAY_GLAMER
};
