//! Vivid (an ability word, CR 207.2c): "the number of colors among permanents you control". Colors are white, blue, black, red
//! and green, so colorless is no color (CR 105.1, 105.2c), and a land is colorless unless something makes it a color. Shinestriker
//! draws that many cards; Rime Chill and Wildvine Pummeler cost that much less.

use cantrip_api::*;

/// The colors among the permanents `player` controls (CR 105.2), each as the layers make it now.
pub fn colors(q: &dyn Query, player: PlayerId) -> ColorSet {
    q.permanents_of(player).into_iter().fold(ColorSet::NONE, |set, id| ColorSet(set.0 | q.chars(id).colors.0))
}

/// How many colors are among the permanents `player` controls, from 0 to 5.
pub fn count(q: &dyn Query, player: PlayerId) -> u8 {
    colors(q, player).iter().count() as u8
}

/// "Vivid - This spell costs {1} less to cast for each color among permanents you control" (CR 601.2f, 700.4): judged once, as the
/// total cost is determined, for the player casting it; generic mana only (CR 118.7a), and colorless is no color. An
/// `own_cost_changes` entry: Rime Chill and Wildvine Pummeler print it.
pub fn cheaper_per_color(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(count(q, caster))
}
