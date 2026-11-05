use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw two cards. Cast Away Doubt deals 2 damage to each player." One damage event for every player still in the game
/// (CR 120.2, 102.2), after the draws.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    let hits: Vec<(ObjectId, Target, i64)> = g.state().apnap().map(|p| (item.object, Target::Player(p), 2)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::CAST_AWAY_DOUBT };
