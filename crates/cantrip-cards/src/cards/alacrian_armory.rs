use cantrip_api::*;
use cantrip_mechanics::statics::{have_vigilance, plus_pt, your_creatures};
use cantrip_mechanics::targets::{self, permanent_of_any_subtype};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Target Mount or Vehicle you control" (CR 115.1, 205.3m, 301.7, 109.5): a permanent with either type, whether or not it
/// is a creature right now, controlled by the ability's controller. Both types are read through this card's words (CR 612.2).
fn mount_or_vehicle_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    permanent_of_any_subtype(q, asker, t, &[Subtype::Mount, Subtype::Vehicle]) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// "Until end of turn, that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a Vehicle."
/// A target no longer legal as it resolves does nothing (CR 608.2b); a permanent that is both a Mount and a Vehicle gets both
/// results, in the order printed (CR 608.2c). A saddled Mount stays saddled until end of turn (CR 702.171b), and a Vehicle
/// that becomes an artifact creature keeps its other types and its Vehicle subtype (CR 205.1b, 301.7).
fn saddle_mount_and_animate_vehicle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let mount = g.words(item.object).subtype(Subtype::Mount);
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    if g.chars(permanent).has_subtype(mount) {
        g.become_saddled(permanent);
    }
    if g.chars(permanent).has_subtype(vehicle) {
        instruction(g, |g| {
            g.add_effect(item.object, permanent, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::EndOfTurn);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, your_creatures, have_vigilance, "Creatures you control get +0/+1 and have vigilance."),
        static_ability(Layer::PtModify, your_creatures, plus_pt::<0, 1>, "Creatures you control get +0/+1 and have vigilance.").continues(),
    ],
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        saddle_mount_and_animate_vehicle,
        "At the beginning of combat on your turn, choose up to one target Mount or Vehicle you control. Until end of turn, that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a Vehicle.",
    )
    .targets(&[target(mount_or_vehicle_you_control).optional()])],
    ..oracle::ALACRIAN_ARMORY
};
