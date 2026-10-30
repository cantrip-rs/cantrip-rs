use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As long as you have 0 or less life, all damage is dealt to you as though its source had infect" (CR 702.90b): asked as
/// the damage is dealt, so damage that takes a positive life total to 0 or less is dealt as life loss (the 2011-06-01 ruling).
fn infect_at_zero_or_less(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller && q.state().player(player).life <= 0
}

pub static CARD: CardDef = CardDef {
    loss_rules: &[cantrip_mechanics::statics::you_cant_lose_from_life],
    infect_damage_to: &[infect_at_zero_or_less],
    ..oracle::PHYREXIAN_UNLIFE
};
