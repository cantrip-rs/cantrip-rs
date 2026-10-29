use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// Porthos, the legendary 1/1 green Dog token Captain Archer creates (CR 111.4).
pub static PORTHOS: CardDef = CardDef {
    name: "Porthos",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Dog]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

fn make_porthos(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &PORTHOS, 1)?;
    Ok(())
}

/// "the number of creature types among non-Borg creatures you control" (CR 205.3m): each creature type is counted
/// once if at least one creature the ability's controller controls has it. A creature with the Borg type is excluded
/// altogether. Borg is read through Archer's words (CR 612.2).
fn creature_types_among_non_borg(q: &dyn Query, source: ObjectId, player: PlayerId) -> usize {
    let borg = q.words(source).subtype(Subtype::Borg);
    let creatures: Vec<ObjectId> = q
        .permanents_of(player)
        .into_iter()
        .filter(|&id| q.chars(id).is(CardType::Creature) && !q.chars(id).has_subtype(borg))
        .collect();
    Subtype::CREATURE_TYPES
        .iter()
        .filter(|&&kind| creatures.iter().any(|&id| q.chars(id).has_subtype(kind)))
        .count()
}

/// "Target creature you control gets +X/+X until end of turn, where X is the number of creature types among non-Borg
/// creatures you control." X is determined once as the ability resolves (CR 608.2h); the pump expires in cleanup
/// (CR 611.2a, 514.2).
fn pump_by_creature_types(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = creature_types_among_non_borg(g, item.object, item.controller) as i16;
    cantrip_mechanics::pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            make_porthos,
            "When Captain Archer enters, create Porthos, a legendary 1/1 green Dog creature token.",
        ),
        ability(
            cost("{G}").and(TAP),
            pump_by_creature_types,
            "Federation — {G}, {T}: Target creature you control gets +X/+X until end of turn, where X is the number of creature types among non-Borg creatures you control. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .sorcery_speed(),
    ],
    ..oracle::CAPTAIN_JONATHAN_ARCHER
};

pub static EXTRA: &[&CardDef] = &[&PORTHOS];
