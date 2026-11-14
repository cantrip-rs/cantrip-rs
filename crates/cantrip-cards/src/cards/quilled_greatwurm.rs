use cantrip_api::*;
use cantrip_mechanics::{cast_from_your_graveyard, targets};
use cantrip_oracle as oracle;

/// "Whenever a creature you control deals combat damage during your turn" (CR 510.2, 603.2, 120.2): what it damaged is not asked
/// about; the creature that dealt it is the trigger's subject and the amount it dealt the ability's
/// `StackItem::matched_damage_amount` (the ability uses `damage_amount` and `sums_per_subject`, so a creature that hurt several
/// blockers at once triggers it once, with the total). The creature is one you control as it dealt the damage, and the turn is
/// your own.
fn creature_you_control_deals_combat_damage_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, amount, kind, .. }
            if kind.is_combat()
                && *amount > 0
                && q.state().turn.active == me.controller
                && q.controller(*from) == me.controller
                && q.chars(*from).is(CardType::Creature) =>
        {
            Some(Fired { subject: Target::Object(*from), data: [*amount as u32, 0] })
        }
        _ => None,
    }
}

/// "Put that many +1/+1 counters on it" (CR 122.1a, 608.2c): the trigger's subject, as many counters as the damage its event
/// dealt. A creature that has left the battlefield gets none ("it must survive to get the counters").
fn that_many_counters_on_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, item.matched_damage_amount() as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, creature_you_control_deals_combat_damage_during_your_turn),
        that_many_counters_on_it,
        "Whenever a creature you control deals combat damage during your turn, put that many +1/+1 counters on it.",
    ).damage_amount().sums_per_subject()],
    // "You may cast this card from your graveyard by removing six counters from among creatures you control in addition to
    // paying its other costs" (CR 118.8, 601.2f): counters of any kinds, from any of your creatures (CR 118.3), at the
    // timing of a creature spell (the ruling: "You must follow all normal timing rules").
    exile_casts: &[ExileCast {
        text: "Cast from your graveyard by removing six counters from among creatures you control",
        additional: remove_counters_among(6, targets::filters::creature),
        ..cast_from_your_graveyard::PERMISSION
    }],
    ..oracle::QUILLED_GREATWURM
};
