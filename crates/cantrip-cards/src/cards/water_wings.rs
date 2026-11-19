use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Flying and hexproof, the keywords the target gains.
const FLYING_AND_HEXPROOF: u128 = Keywords::of(&[Keyword::Flying, Keyword::Hexproof]).0;

/// "Until end of turn, target creature you control has base power and toughness 4/4 and gains flying and hexproof." (CR 611.2a, 613.4b,
/// 702.9, 702.11): one sentence joined by "and", so one instruction (CR 608.2c); nothing if the target is no longer legal (CR 608.2b).
fn base_4_4_and_gains_flying_and_hexproof(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        effects::set_first_target_base_pt::<4, 4>(g, item)?;
        effects::first_target_gains_until_end_of_turn::<FLYING_AND_HEXPROOF>(g, item)
    })
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], base_4_4_and_gains_flying_and_hexproof),
    ..oracle::WATER_WINGS
};
