use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies, amass Goblins 4" (CR 701.47a, 603.10a): a leaves-the-battlefield ability, so it triggers on the Pair
/// dying and amasses for the controller it last had; four +1/+1 counters go on an Army that player controls, and a 0/0 black
/// Goblin Army token is created first if there is none.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, amass_goblins::<4>, "When this creature dies, amass Goblins 4.")],
    ..oracle::FEARSOME_GOBLIN_PAIR
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
