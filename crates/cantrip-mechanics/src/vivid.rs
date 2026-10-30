//! Vivid (an ability word, CR 207.2c): "the number of colors among permanents you control". Colors are white, blue, black, red
//! and green, so colorless is no color (CR 105.1, 105.2c), and a land is colorless unless something makes it a color. Shinestriker
//! draws that many cards; Rime Chill costs that much less.

use cantrip_api::*;

/// The colors among the permanents `player` controls (CR 105.2), each as the layers make it now.
pub fn colors(q: &dyn Query, player: PlayerId) -> ColorSet {
    q.permanents_of(player).into_iter().fold(ColorSet::NONE, |set, id| ColorSet(set.0 | q.chars(id).colors.0))
}

/// How many colors are among the permanents `player` controls, from 0 to 5.
pub fn count(q: &dyn Query, player: PlayerId) -> u8 {
    colors(q, player).iter().count() as u8
}
