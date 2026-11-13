use cantrip_api::*;
use cantrip_mechanics::{artifacts_you_control, effects, targets, tokens};
use cantrip_oracle as oracle;

pub use cantrip_mechanics::tokens::THOPTER;

/// "Choose target opponent. Create two 1/1 colorless Thopter artifact creature tokens with flying. Then the chosen player loses
/// X life and you gain X life, where X is the number of artifacts you control." The chosen player is the spell's one target
/// (CR 115.1, 601.2c). The tokens are made first, under your control (CR 608.2c, 111.2), and X is counted once, as the last
/// sentence resolves (CR 608.2h), so it counts them. The opponent is the spell's only target, so if it is no longer legal the
/// spell does not resolve at all (CR 608.2b) and nothing is made, lost or gained.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &THOPTER, 2)?;
    g.end_sentence();
    let x = artifacts_you_control(g, item.controller) as i32;
    effects::target_player_loses_life_you_gain_life(g, item, x)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], effect), ..oracle::HAUNT_THE_NETWORK };

pub static EXTRA: &[&CardDef] = &[&THOPTER];
