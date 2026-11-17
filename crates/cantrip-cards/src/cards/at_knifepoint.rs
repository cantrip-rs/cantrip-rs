use cantrip_api::*;
use cantrip_mechanics::{outlaw, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "During your turn, outlaws you control have first strike." (CR 700.12a, 604.1, 613.1f): an Assassin, Mercenary, Pirate,
/// Rogue or Warlock under this enchantment's controller, judged on what the earlier layers made it, while that player is the
/// active player (CR 102.1).
fn outlaws_you_control_during_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let you = q.state().obj(source).controller;
    q.state().turn.active == you && q.controller(affected) == you && outlaw::is_outlaw(so_far)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        outlaws_you_control_during_your_turn,
        statics::have_first_strike,
        "During your turn, outlaws you control have first strike. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.)",
    )],
    abilities: &[triggered(
        triggers::YOU_COMMIT_A_CRIME,
        tokens::create_a_mercenary,
        "Whenever you commit a crime, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\" This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::AT_KNIFEPOINT
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
