use cantrip_api::*;
use cantrip_mechanics::{double, greatest_power, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the greatest power among creatures you control" (CR 601.2f, 118.7a, 208.1): worked out as the
/// cost is determined, from the creatures as they are then. The reduction takes off generic mana only (CR 118.7a), never a colored symbol.
fn costs_less_by_the_greatest_power(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(greatest_power::among_creatures_you_control(q, player))
}

/// The token: a green Fungus Dinosaur creature, its size given as it is made (`tokens::create_sized`), so it states none.
static FUNGUS_DINOSAUR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    ..token!(Fungus Dinosaur)
};

/// The creature that died, as the batch's member: the trigger of "a nontoken creature you control dies" with the creature as its subject, so the
/// creatures that die together are the members of the one trigger (CR 603.2c) and each can be looked back on (CR 603.10a).
fn nontoken_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::nontoken_creature_you_control_dies(q, me, event)?;
    let Event::ZoneChange { old, .. } = event else { return None };
    Fired::about(Target::Object(*old))
}

/// "Whenever one or more nontoken creatures you control die, create a green Fungus Dinosaur creature token with base power and toughness each
/// equal to the total power of those creatures" (CR 700.4, 603.2c, 603.10a, 111.4): the creatures of the batch that died together, each with the
/// power it last had on the battlefield (the ruling). A negative power is added as it is (CR 107.1b: a calculation that needs a negative
/// value uses it), and a negative total is not raised to 0, since the effect sets the token's power and toughness to it (107.1b's
/// exception). The token is that size as it is made, never a 0/0 for a moment; a total of 0 or less makes a token that state-based actions
/// put into the graveyard (CR 704.5f).
fn create_a_fungus_dinosaur(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let total: i32 = g.event_objects(item).iter().map(|&creature| targets::power_or_last_known(g, creature)).sum();
    tokens::create_sized(g, item.object, item.controller, &FUNGUS_DINOSAUR, (total, total), 1)?;
    Ok(())
}

/// "{2}, {T}: Double target creature's power until end of turn" (CR 701.10b-c): +X/+0 for the power it has as the ability resolves. Nothing
/// happens if the target is no longer legal (CR 608.2b).
fn double_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    double::power(g, item.object, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[costs_less_by_the_greatest_power],
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, nontoken_creature_you_control_dies),
            create_a_fungus_dinosaur,
            "Whenever one or more nontoken creatures you control die, create a green Fungus Dinosaur creature token with base power and toughness each equal to the total power of those creatures.",
        )
        .once_per_batch(),
        ability(cost("{2}, {T}"), double_power, "{2}, {T}: Double target creature's power until end of turn.").targets(&[targets::CREATURE]),
    ],
    ..oracle::THE_SKULLSPORE_NEXUS
};
pub static EXTRA: &[&CardDef] = &[&FUNGUS_DINOSAUR];
