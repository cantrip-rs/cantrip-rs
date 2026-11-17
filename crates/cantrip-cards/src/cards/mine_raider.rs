use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{outlaw, tokens};
use cantrip_oracle as oracle;

/// "If you control another outlaw" (CR 700.12a): a permanent other than the source, controlled by the ability's controller, with an
/// outlaw creature type as it is now. An opponent's outlaw does not count. As an intervening "if" (CR 603.4) it is asked as the ability
/// would trigger and again as it resolves.
fn you_control_another_outlaw(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| id != asker.source && outlaw::is_outlaw(&q.chars(id)))
}

/// "When this creature enters, if you control another outlaw, create a Treasure token." (CR 603.4, 701.7a)
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tokens::create_a_treasure,
        "When this creature enters, if you control another outlaw, create a Treasure token.",
    )
    .condition(you_control_another_outlaw)],
    ..oracle::MINE_RAIDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
