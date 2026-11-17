use cantrip_api::*;
use cantrip_mechanics::{effects, outlaw, targets};
use cantrip_oracle as oracle;

/// "Whenever another outlaw you control enters" (CR 603.6a, 700.12): a permanent other than this one that is an outlaw as it
/// entered (`outlaw::is_outlaw`: an Assassin, Mercenary, Pirate, Rogue or Warlock), under your control.
fn another_outlaw_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if *new != me.source && to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && outlaw::is_outlaw(&q.chars(*new)) => {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_outlaw_you_control_enters),
        effects::damage_first_target::<1>,
        "Whenever another outlaw you control enters, Vial Smasher deals 1 damage to target opponent. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.)",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::VIAL_SMASHER_GLEEFUL_GRENADIER
};
