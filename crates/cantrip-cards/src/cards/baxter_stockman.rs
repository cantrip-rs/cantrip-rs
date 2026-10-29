use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Baxter Stockman enters, create a 1/1 colorless Robot artifact creature token" (CR 603.6a, 111.1): the controller of
/// the ability creates it.
fn robot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ROBOT_1_1, 1)?;
    Ok(())
}

/// "Target artifact creature you control" (CR 115.1).
fn artifact_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.controller(id) == asker.controller && {
        let c = q.chars(id);
        c.is(CardType::Artifact) && c.is(CardType::Creature)
    })
}

/// "Gets +3/+0 and gains first strike and vigilance until end of turn": one instruction (CR 608.2c). Nothing happens if it is no
/// longer a legal target (CR 608.2b).
fn boost(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 3, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Vigilance);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, robot, "When Baxter Stockman enters, create a 1/1 colorless Robot artifact creature token."),
        triggered(
            triggers::BEGINNING_OF_YOUR_COMBAT,
            boost,
            "At the beginning of combat on your turn, target artifact creature you control gets +3/+0 and gains first strike and vigilance until end of turn.",
        )
        .targets(&[target(artifact_creature_you_control)]),
    ],
    ..oracle::BAXTER_STOCKMAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::ROBOT_1_1];
