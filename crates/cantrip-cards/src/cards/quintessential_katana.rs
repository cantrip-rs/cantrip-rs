use cantrip_api::*;
use cantrip_mechanics::attach::{self, attached_object, equip};
use cantrip_mechanics::effects::gain_life;
use cantrip_mechanics::triggers::THIS_DEALS_COMBAT_DAMAGE;
use cantrip_oracle as oracle;

/// "Whenever a Ninja you control enters" (CR 603.6a): the Ninja under your control as it entered, judged as it is on the
/// battlefield. The entering Ninja is the trigger's subject, "it".
fn ninja_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && q.controller(*new) == me.controller
                && q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Ninja)) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "You may attach this Equipment to it" (CR 701.3a): an Equipment already attached elsewhere moves to it, with a new timestamp (CR 701.3c).
fn may_attach_to_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    attach::may_attach_to(g, item.controller, item.object, creature, Purpose::Attach)
}

/// "Untap it and you gain 2 life": "it" is the creature that has the ability, the source of the ability on the stack.
fn untap_it_and_gain_2_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if cantrip_mechanics::targets::permanent(g, item.object) {
        g.untap(item.object);
    }
    gain_life(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, ninja_you_control_enters),
            may_attach_to_it,
            "Whenever a Ninja you control enters, you may attach this Equipment to it.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    delayed: &[triggered(THIS_DEALS_COMBAT_DAMAGE, untap_it_and_gain_2_life, "Whenever this creature deals combat damage, untap it and you gain 2 life.")
        .once_per_batch()],
    statics: &[
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, cantrip_mechanics::statics::grant_delayed_ability, "Equipped creature has \"Whenever this creature deals combat damage, untap it and you gain 2 life.\""),
    ],
    ..oracle::QUINTESSENTIAL_KATANA
};
