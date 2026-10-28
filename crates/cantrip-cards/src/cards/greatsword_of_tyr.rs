use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::EQUIPPED_CREATURE_ATTACKS_SUBJECT;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on it and tap up to one target creature defending player controls." The counter goes on the
/// equipped creature that attacked - nothing, once it has left the battlefield (a new object, CR 400.7; `add_counters`
/// skips it) - whatever the target does; a target no longer legal is not tapped (CR 608.2b).
fn counter_and_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    if let Some(defender) = targets::object(g, item, 0) {
        g.tap(defender);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            EQUIPPED_CREATURE_ATTACKS_SUBJECT,
            counter_and_tap,
            "Whenever equipped creature attacks, put a +1/+1 counter on it and tap up to one target creature defending player controls.",
        )
        .targets(&[up_to_one(targets::creature_defending_player_controls)]),
        equip(mana("{W}"), "Equip {W}"),
    ],
    ..oracle::GREATSWORD_OF_TYR
};
