use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/4 colorless Wall creature token with defender." (CR 111.1.)
fn wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WALL_0_4_DEFENDER, 1)?;
    Ok(())
}

/// "Super-Skrull gets +4/+4 until end of turn." (CR 611.2a): on him if he is still on the battlefield.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump(g, item.object, item.object, 4, 4);
    Ok(())
}

/// "Super-Skrull deals 4 damage to target creature." (CR 120.3): dealt by him, as he last was if he has left the battlefield
/// (CR 113.7a); nothing if the creature is no longer a legal target (CR 608.2b).
fn four_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

/// Flying is a core keyword read from its characteristics. Each activated ability needs one mana of a different color of the
/// other four (white, green, red, blue), and can be activated at instant speed.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}{W}"), wall, "{2}{W}: Create a 0/4 colorless Wall creature token with defender."),
        ability(cost("{3}{G}"), grow, "{3}{G}: Super-Skrull gets +4/+4 until end of turn."),
        ability(cost("{4}{R}"), four_damage, "{4}{R}: Super-Skrull deals 4 damage to target creature.").targets(&[targets::CREATURE]),
        ability(
            cost("{5}{U}"),
            |g, item| {
                effects::target_player_draws(g, item, 0, 4);
                Ok(())
            },
            "{5}{U}: Target player draws four cards.",
        )
        .targets(&[targets::PLAYER]),
    ],
    ..oracle::SUPER_SKRULL
};

pub static EXTRA: &[&CardDef] = &[&tokens::WALL_0_4_DEFENDER];
