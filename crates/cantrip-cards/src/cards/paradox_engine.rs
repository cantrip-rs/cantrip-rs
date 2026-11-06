use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Untap all nonland permanents you control" (CR 701.26b): everything its controller then controls that is not a
/// land, with no choice and no count.
fn untap_nonland(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let nonland: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| !g.chars(id).is(CardType::Land)).collect();
    g.untap_together(&nonland);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::YOU_CAST_A_SPELL, untap_nonland, "Whenever you cast a spell, untap all nonland permanents you control.")],
    ..oracle::PARADOX_ENGINE
};
