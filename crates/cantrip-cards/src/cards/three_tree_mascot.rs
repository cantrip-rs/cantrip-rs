use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Changeling" comes from the printed keywords (CR 702.73a). "{1}: Add one mana of any color. Activate only once each turn."
/// is a mana ability (CR 605.1a) that is paid from the pool and counts its activations each turn (CR 602.5b).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(cost("{1}"), add_any_color(1), "{1}: Add one mana of any color. Activate only once each turn.").once_each_turn()],
    ..oracle::THREE_TREE_MASCOT
};
