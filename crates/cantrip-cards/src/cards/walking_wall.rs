use cantrip_api::*;
use cantrip_oracle as oracle;

/// The pump and permission last through this turn (CR 608.2, 611.2a).
fn pump_and_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::Pump(3, -1), Duration::EndOfTurn);
        g.add_effect(item.object, item.object, EffectKind::CanAttackDespiteDefender, Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}"),
        pump_and_attack,
        "{3}: This creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender. Activate only once each turn.",
    ).once_each_turn()],
    ..oracle::WALKING_WALL
};
