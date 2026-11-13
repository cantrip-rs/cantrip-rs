use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}: For as long as this creature remains tapped, gain control of target creature of an opponent's choice they control." (CR 115.1).
fn target_chooser_0(q: &dyn Query, asker: &Asker) -> Vec<PlayerId> {
    q.state().opponents(asker.controller).collect::<Vec<PlayerId>>()
}

/// "{T}: For as long as this creature remains tapped, gain control of target creature of an opponent's choice they control." (CR 115.1).
fn as_long_as_creature_remains_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && q.target_chooser() == Some(q.controller(id))))
    )
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
            .targets(
                &[
                    target(as_long_as_creature_remains_target_0)
                        .chosen_by_one_of(target_chooser_0),
                ],
            ),
    ],
    may_not_untap: true,
    ..oracle::PREACHER
};
