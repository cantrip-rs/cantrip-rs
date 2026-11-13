use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "Target creature of an opponent's choice they control" (CR 115.1, 601.7a, 602.3a): a creature the opponent who chooses
/// the target controls, that opponent being the slot's (`Query::slot_opponent`), as it is chosen and as it resolves.
fn creature_the_choosing_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::controlled_by_slot_opponent(q, id) && filters::creature(q, asker, id))
}

/// "{T}: For as long as this creature remains tapped, gain control of target creature of an opponent's choice they control." (CR 608.2, 611.2a).
fn as_long_as_creature_remains(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.gain_control(item.object, creature, item.controller, Duration::WhileSourceTapped);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                as_long_as_creature_remains,
                "{T}: For as long as this creature remains tapped, gain control of target creature of an opponent's choice they control.",
            )
            .targets(&[target(creature_the_choosing_opponent_controls).of_an_opponents_choice()]),
    ],
    may_not_untap: true,
    ..oracle::PREACHER
};
