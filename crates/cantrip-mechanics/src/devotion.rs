//! Devotion (CR 700.5), an ability word's measure: Thassa's Oracle, Nykthos, Shrine to Nyx.
use cantrip_api::*;

/// Mana symbols of `color` in a permanent's or spell's mana cost (CR 700.5, 202.2; Namor the Sub-Mariner counts a spell's): hybrid ({U/R}, {2/U}) and Phyrexian symbols of the
/// color each count once, numeric and other colors' symbols not. A copy counts the cost it copies (CR 707.2, 700.5a),
/// none if it has none (CR 707.9b).
pub fn symbols_of(q: &dyn Query, id: ObjectId, color: Color) -> u32 {
    q.mana_symbols(id, color)
}

/// "Your devotion to [color]" (CR 700.5): the mana symbols of `color` among the mana costs of permanents `player`
/// controls, counted now.
pub fn devotion(q: &dyn Query, player: PlayerId, color: Color) -> u32 {
    q.permanents_of(player).into_iter().map(|id| symbols_of(q, id, color)).sum()
}
