use cantrip_api::*;
use cantrip_mechanics::triggers::{creature_you_control_dies, THIS_ENTERS};
use cantrip_mechanics::{damage_target, effects, fight, targets};
use cantrip_oracle as oracle;

/// "Whenever a legendary creature you control dies" (CR 700.4, 603.10a): a creature you control dying, as `creature_you_control_dies`
/// judges it, that was legendary as it last was on the battlefield. This permanent leaving in the same event still sees it, and each
/// of several that die together triggers it. The ability is about that creature, whose power it reads.
fn legendary_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if q.chars(*old).supertypes.legendary && creature_you_control_dies(q, me, event).is_some() => {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// "Rakdos Joins Up deals damage equal to that creature's power to target opponent" (CR 120.3, 608.2h): the power the creature had as it
/// last existed on the battlefield, none if that was zero or less (CR 120.8). This permanent is the source, as it last existed if it has
/// left (CR 113.7a).
fn damage_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        let power = fight::damage_equal_to_power(g, creature);
        damage_target(g, item, 0, power);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            effects::return_first_target_to_battlefield_with_two_plus_one_counters,
            "When Rakdos Joins Up enters, return target creature card from your graveyard to the battlefield with two additional +1/+1 counters on it.",
        )
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, legendary_creature_you_control_dies),
            damage_equal_to_its_power,
            "Whenever a legendary creature you control dies, Rakdos Joins Up deals damage equal to that creature's power to target opponent.",
        )
        .targets(&[targets::OPPONENT]),
    ],
    ..oracle::RAKDOS_JOINS_UP
};
