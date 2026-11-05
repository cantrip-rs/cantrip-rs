use cantrip_api::*;
use cantrip_mechanics::statics::{damage_cant_be_prevented, players_cant_gain_life};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "It deals damage to each player equal to the number of nonbasic lands that player controls" (CR 120.2, 603.2c): each
/// player's own count, as the ability resolves, dealt to every player at once. A land with no basic supertype is nonbasic
/// (CR 205.4a); a player who controls none is dealt no damage (CR 120.8).
fn burn_each_player(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, i64)> = g
        .state()
        .apnap()
        .map(|p| {
            let nonbasic = g.permanents_of(p).into_iter().filter(|&id| {
                let c = g.chars(id);
                c.is(CardType::Land) && !c.supertypes.basic
            });
            (item.object, Target::Player(p), i64::from(u16::try_from(nonbasic.count()).unwrap_or(u16::MAX)))
        })
        .filter(|&(_, _, n)| n > 0)
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cant_gain_life: &[players_cant_gain_life],
    unpreventable_damage: &[damage_cant_be_prevented],
    abilities: &[triggered(THIS_ENTERS, burn_each_player, "When this creature enters, it deals damage to each player equal to the number of nonbasic lands that player controls.")],
    ..oracle::SUNSPINE_LYNX
};
