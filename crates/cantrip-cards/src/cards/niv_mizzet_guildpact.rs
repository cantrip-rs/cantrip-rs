use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers};
use cantrip_oracle as oracle;

/// "X is the number of different color pairs among permanents you control that are exactly two colors" (CR 105.1, 202.2): every
/// permanent its controller controls that has exactly two colors as the layers have it now, counted by which two colors, so two
/// Ravnica-style gold permanents of the same pair count once, and a permanent with three or more colors or with one or none
/// counts for nothing (the rulings). Counted as the ability resolves (CR 608.2h), not as it was put on the stack.
fn different_color_pairs(q: &dyn Query, player: PlayerId) -> u32 {
    let mut seen: Vec<ColorSet> = Vec::new();
    for id in q.state().battlefield().iter().copied().filter(|&id| q.controller(id) == player) {
        let colors = q.chars(id).colors;
        if colors.iter().count() == 2 && !seen.contains(&colors) {
            seen.push(colors);
        }
    }
    seen.len() as u32
}

/// "It deals X damage to any target, target player draws X cards, and you gain X life" (CR 120.3, 121.1, 119.3): each target is
/// judged alone as the ability resolves (CR 608.2b), and X is worked out once, at resolution, for all three parts.
fn damage_draw_and_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = different_color_pairs(g, item.controller);
    damage_target(g, item, 0, x);
    if let Some(player) = targets::player(g, item, 1) {
        g.draw(player, x);
    }
    g.gain_life(item.controller, x as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Hexproof from multicolored" (CR 702.11d): a source of two or more colors an opponent controls can't target it.
    hexproof_from: HexproofFrom::MULTICOLORED,
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        damage_draw_and_gain,
        "Whenever Niv-Mizzet deals combat damage to a player, it deals X damage to any target, target player draws X cards, and you gain X life, where X is the number of different color pairs among permanents you control that are exactly two colors.",
    )
    .targets(&[targets::ANY, targets::PLAYER])],
    ..oracle::NIV_MIZZET_GUILDPACT
};
