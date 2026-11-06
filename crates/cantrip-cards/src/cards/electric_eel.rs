use cantrip_api::*;
use cantrip_mechanics::{effects, pump, triggers};
use cantrip_oracle as oracle;

/// The pump and damage are one instruction; damage still uses the source's last known information if it left (CR 113.7a, 608.2c).
fn pump_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, 2, 0);
        g.damage(item.object, Target::Player(item.controller), 1);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::source_deals_one_to_controller,
            "When this creature enters, it deals 1 damage to you.",
        ),
        ability(
            cost("{R}{R}"),
            pump_and_damage,
            "{R}{R}: This creature gets +2/+0 until end of turn and deals 1 damage to you.",
        ),
    ],
    ..oracle::ELECTRIC_EEL
};
