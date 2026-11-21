use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::has_delayed_ability;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap target creature. Return Trusty Boomerang to its owner's hand." (CR 608.2c): in that order. "Trusty Boomerang" is the
/// Equipment that gave the creature this ability (CR 113.10, 613.1f), which the item keeps as it was activated
/// (`AbilityAcquisition::granter`; CR 113.7a: the ability is independent of both). Unattached since, or without its creature,
/// it is still that object; once it has changed zones it is a new object (CR 400.7) and nothing is returned.
fn tap_target_creature_then_return_the_boomerang(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_targets(g, item)?;
    if item.instance.granter != ObjectId::NONE {
        g.return_to_hand(&[item.instance.granter]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}")],
    // The ability the equipped creature has (CR 613.1f), its own, so its controller controls and pays for it (CR 113.8).
    delayed: &[ability(cost("{1}").and(TAP), tap_target_creature_then_return_the_boomerang, "{1}, {T}: Tap target creature. Return Trusty Boomerang to its owner's hand.")
        .targets(&[targets::CREATURE])],
    statics: &[static_ability(
        Layer::Ability,
        attached_object,
        has_delayed_ability::<0>,
        "Equipped creature has \"{1}, {T}: Tap target creature. Return Trusty Boomerang to its owner's hand.\"",
    )],
    ..oracle::TRUSTY_BOOMERANG
};
