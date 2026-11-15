use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, you may mill two cards." (CR 701.17a, 701.17b)
fn may_mill_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_mill(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        may_mill_two,
        "When this creature enters, you may mill two cards. (You may put the top two cards of your library into your graveyard.)",
    )],
    ..oracle::DAGGERFANG_DUO
};
