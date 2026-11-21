use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card in your graveyard gains harmonize until end of turn. Its harmonize cost is equal to its
/// mana cost" (CR 702.180a): its controller may cast it from the graveyard this turn for its mana cost, tapping a creature
/// to reduce the generic part by that creature's power, and it is exiled instead of going anywhere else as it leaves the
/// stack (CR 702.180a).
fn grant_harmonize(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(card) = item.target(0)
    {
        g.allow_play(card, item.controller, PlayGrant { expiry: Expiry::EndOfTurn, harmonize: true, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, grant_harmonize, "When this creature enters, target instant or sorcery card in your graveyard gains harmonize until end of turn. Its harmonize cost is equal to its mana cost.")
        .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])],
    ..oracle::SONGCRAFTER_MAGE
};
