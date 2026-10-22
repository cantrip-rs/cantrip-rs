use cantrip_api::*;
use cantrip_mechanics::{power_up, tokens};
use cantrip_oracle as oracle;

/// The 2/2 colorless Robot Villain artifact creature token (CR 111.4: named for its subtypes).
pub static ROBOT_VILLAIN: CardDef = CardDef {
    name: "Robot Villain Token",
    token_name: Some(&[Subtype::Robot, Subtype::Villain]),
    has_cost: false,
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Robot, Subtype::Villain]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "Put two +1/+1 counters on this creature and create a 2/2 colorless Robot Villain artifact creature token." The
/// token is made even if the creature has left, and only the counters have nothing to go on (CR 609.3).
fn counters_and_robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 2);
    tokens::create(g, item.controller, &ROBOT_VILLAIN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[power_up::ability(
        cost("{6}"),
        counters_and_robot,
        "Power-up \u{2014} {6}: Put two +1/+1 counters on this creature and create a 2/2 colorless Robot Villain artifact creature token. (Activate each power-up ability only once. Reduce the cost by its mana cost if it entered this turn.)",
    )],
    ..oracle::ULTRON_DRONE
};

pub static EXTRA: &[&CardDef] = &[&ROBOT_VILLAIN];
