use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ATTACKS;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "That creature deals 1 damage to each opponent": the attacking creature is the source (CR 120.3), as it last was if it has
/// left the battlefield by now (CR 113.7a, 608.2h); one damage event for each opponent still in the game, dealt together.
fn attacker_deals_1_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(attacker) = item.subject else { return Ok(()) };
    let hits: Vec<(ObjectId, Target, i64)> = g.state().opponents(item.controller).map(|p| (attacker, Target::Player(p), 1)).collect();
    g.damage_together(&hits);
    Ok(())
}

/// "Create a 2/2 colorless Wizard Soldier creature token named Cadet. Then creatures you control gain haste until end of turn":
/// the creatures on the battlefield as it resolves (CR 611.2c), the Cadet among them.
fn cadet_then_haste(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::CADET, 1)?;
    g.end_sentence();
    effects::creatures_you_control_gain(g, item, &[Keyword::Haste]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(CREATURE_YOU_CONTROL_ATTACKS, attacker_deals_1_to_each_opponent, "Whenever a creature you control attacks, that creature deals 1 damage to each opponent."),
        ability(cost("{4}"), cadet_then_haste, "{4}: Create a 2/2 colorless Wizard Soldier creature token named Cadet. Then creatures you control gain haste until end of turn."),
    ],
    ..oracle::INGRIS_STINGERQUILL
};

pub static EXTRA: &[&CardDef] = &[&tokens::CADET];
