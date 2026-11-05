use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Slagstorm deals 3 damage to each creature."
fn creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 3);
    Ok(())
}

/// "Slagstorm deals 3 damage to each player": one event for every player still in the game (CR 120.2).
fn players(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, i64)> = g.state().apnap().map(|p| (item.object, Target::Player(p), 3)).collect();
    g.damage_together(&hits);
    Ok(())
}

/// Choose one (CR 700.2).
static MODAL: ModalDef = modes(&[
    mode(creatures, "Slagstorm deals 3 damage to each creature."),
    mode(players, "Slagstorm deals 3 damage to each player."),
])
.choose(1, 1);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SLAGSTORM };
