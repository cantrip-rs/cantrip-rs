use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a 0/4 colorless Wall creature token with defender." (CR 111.1.)
fn wall(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WALL_0_4_DEFENDER, 1)?;
    Ok(())
}

/// Flying is a core keyword read from its characteristics. Each activated ability needs one mana of a different color of the
/// other four (white, green, red, blue), and can be activated at instant speed.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}{W}"), wall, "{2}{W}: Create a 0/4 colorless Wall creature token with defender."),
        ability(cost("{3}{G}"), effects::pump_self::<4, 4>, "{3}{G}: Super-Skrull gets +4/+4 until end of turn."),
        ability(cost("{4}{R}"), effects::damage_first_target::<4>, "{4}{R}: Super-Skrull deals 4 damage to target creature.").targets(&[targets::CREATURE]),
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
