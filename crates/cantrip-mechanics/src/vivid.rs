//! Vivid (an ability word, CR 207.2c): count the colors among the permanents a player controls (CR 105.2).
//! Explosive Prodigy and Squawkroaster use this same count.

use cantrip_api::*;

/// The number of distinct colors among permanents `player` controls, using current characteristics: colorless adds none.
pub fn count(q: &dyn Query, player: PlayerId) -> u8 {
    count_colors(q.state(), player, |id| q.chars(id))
}

/// The same count during a continuous effect, using the characteristics from the preceding layers.
pub fn count_in_layers(q: &dyn LayerQuery, player: PlayerId) -> u8 {
    count_colors(q.state(), player, |id| q.chars(id))
}

fn count_colors(state: &GameState, player: PlayerId, chars: impl Fn(ObjectId) -> Chars) -> u8 {
    let colors = state
        .battlefield()
        .iter()
        .filter(|&&id| state.obj(id).controller == player)
        .fold(0, |colors, &id| colors | chars(id).colors.0);
    colors.count_ones() as u8
}
