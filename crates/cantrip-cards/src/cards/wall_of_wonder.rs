use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{2}{U}{U}: This creature gets +4/-4 until end of turn and can attack this turn as though it didn't have defender." (CR 608.2, 611.2a).
fn creature_gets_4_4_can(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::Pump(4, -4), Duration::EndOfTurn);
        g.add_effect(item.object, item.object, EffectKind::CanAttackDespiteDefender, Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{U}{U}"),
            creature_gets_4_4_can,
            "{2}{U}{U}: This creature gets +4/-4 until end of turn and can attack this turn as though it didn't have defender.",
        ),
    ],
    ..oracle::WALL_OF_WONDER
};
