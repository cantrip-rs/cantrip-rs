use cantrip_api::*;
use cantrip_mechanics::modified::is_modified;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a modified creature you control leaves the battlefield" (CR 603.6c, 603.10a, 700.9): a creature that was, as it last
/// was on the battlefield, under this artifact's controller and modified (a counter on it, an Equipment attached, or an Aura its
/// controller controlled attached), whichever zone it goes to.
fn modified_creature_you_control_leaves(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, .. }
            if from.is(ZoneKind::Battlefield)
                && q.chars(*old).is(CardType::Creature)
                && q.controller(*old) == me.controller
                && is_modified(q, *old) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_plus_one_counters::<2>),
    abilities: &[
        ability(
            cost("{T}"),
            effects::move_a_plus_one_counter_from_this_onto_target,
            "{T}: Move a +1/+1 counter from this artifact onto target creature you control. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed(),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, modified_creature_you_control_leaves),
            effects::plus_one_counter_on_this,
            "Whenever a modified creature you control leaves the battlefield, put a +1/+1 counter on this artifact.",
        ),
    ],
    ..oracle::COSTUME_CLOSET
};
