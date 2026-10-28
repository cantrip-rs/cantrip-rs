use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip, equip_quality};
use cantrip_mechanics::statics::grant_delayed_ability;
use cantrip_mechanics::triggers::PROWESS;
use cantrip_oracle as oracle;

/// "Equip Wizard" (CR 702.6a): the target is a Wizard creature you control, "Wizard" read through the words of this card's
/// text (CR 612).
fn wizard_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id)
        && q.controller(id) == asker.controller
        && q.chars(id).is(CardType::Creature)
        && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Wizard)))
}

/// "If a triggered ability of equipped creature triggers, that ability triggers an additional time." (CR 603.2d): an ability
/// the equipped creature has, whoever controls it, that is its own (a delayed triggered ability is no ability the creature has,
/// CR 603.7), the creature being what the Staff is attached to - as it last was for an ability that looks back
/// (`Triggering::permanent`, CR 603.10a). The additional trigger is not a replacement effect (the ruling), and choices such
/// as targets are made for each separately.
fn equipped_creature_triggers_again(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.permanent && q.obj(asker.source).attached_to == Some(triggering.source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        equip_quality(mana("{1}"), &[target(wizard_you_control)], "Equip Wizard {1}"),
        equip(mana("{3}"), "Equip {3}"),
    ],
    delayed: &[PROWESS],
    statics: &[static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has prowess. (Whenever its controller casts a noncreature spell, that creature gets +1/+1 until end of turn.)")],
    additional_triggers: &[equipped_creature_triggers_again],
    ..oracle::WIZARDS_STAFF
};
