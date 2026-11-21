use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever another creature you control or a land you control is put into a graveyard from the battlefield" (CR 603.10a): a
/// creature other than Long Feng, or a land, as it last was on the battlefield and under the controller it last had.
fn another_creature_or_a_land_you_control_to_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. } if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.controller(*old) == me.controller => {
            let c = q.chars(*old);
            if c.is(CardType::Land) || (c.is(CardType::Creature) && *old != me.source) { Fired::PLAIN } else { None }
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_creature_or_a_land_you_control_to_graveyard),
        effects::plus_one_counters_on_target::<1>,
        "Whenever another creature you control or a land you control is put into a graveyard from the battlefield, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::LONG_FENG_GRAND_SECRETARIAT
};
