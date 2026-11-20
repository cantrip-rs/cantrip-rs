use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{ferocious, pump};
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 until end of turn and can attack this turn as though it didn't have defender" (CR 611.2a, 609.4, 702.3b):
/// two continuous effects of one instruction, both lasting through this turn and ending if it leaves the battlefield (CR 400.7).
fn pump_and_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, 1, 0);
        g.add_effect(item.object, item.object, EffectKind::CanAttackDespiteDefender, Duration::EndOfTurn);
    });
    Ok(())
}

/// "...if you control a creature with power 4 or greater": an intervening "if" (CR 603.4), asked as combat begins and again as the
/// ability resolves. The Witchstalker counts itself when its power is 4 or more (it isn't "another").
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        pump_and_attack,
        "At the beginning of combat on your turn, if you control a creature with power 4 or greater, this creature gets +1/+0 until end of turn and can attack this turn as though it didn't have defender.",
    )
    .condition(ferocious::you_control_asked)],
    ..oracle::TERRITORIAL_WITCHSTALKER
};
