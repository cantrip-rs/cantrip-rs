use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// I — Aerial Blast: "This creature deals 4 damage to target tapped creature an opponent controls." (CR 120.3, 714.2b): the
/// Saga deals it, as it last existed if it has left the battlefield (CR 608.2h); nothing if the creature is no longer a legal
/// target, untapped or gone (CR 608.2b).
fn aerial_blast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

const SLIPSTREAM: &str = "II, III \u{2014} Slipstream \u{2014} Another target creature you control gets +1/+0 and gains flying until end of turn.";

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, aerial_blast, "I \u{2014} Aerial Blast \u{2014} This creature deals 4 damage to target tapped creature an opponent controls.")
            .targets(&[targets::TAPPED_CREATURE_AN_OPPONENT_CONTROLS]),
        chapter(2, effects::target_creature_gets_and_gains_flying::<1, 0>, SLIPSTREAM).targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
        chapter(3, effects::target_creature_gets_and_gains_flying::<1, 0>, SLIPSTREAM).targets(&[targets::OTHER_CREATURE_YOU_CONTROL]),
    ],
    ..oracle::SUMMON_PRIMAL_GARUDA
};
