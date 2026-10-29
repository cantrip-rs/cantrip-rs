use cantrip_api::*;
use cantrip_mechanics::{haste_until_end_of_turn, tokens};
use cantrip_oracle as oracle;

fn make_a_robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ROBOT_1_1, 1)?;
    Ok(())
}

/// Creature tokens you control gain haste until end of turn: the set is fixed as the ability resolves, so a token made
/// afterwards is not reached (CR 611.2c).
fn tokens_gain_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.obj(id).token && g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for id in tokens {
            haste_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::ARTIFACT_SPELL_YOU_CAST, make_a_robot, "Whenever you cast an artifact spell, create a 1/1 colorless Robot artifact creature token."),
        ability(cost("{R}, {T}"), tokens_gain_haste, "{R}, {T}: Creature tokens you control gain haste until end of turn."),
    ],
    ..oracle::RAVENOUS_ROBOTS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ROBOT_1_1];
