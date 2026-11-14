use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Players play with their hands revealed." (CR 604.1).
fn hand_revealed_0(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}

pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::HandRevealed(hand_revealed_0)],
    ..oracle::REVELATION
};
