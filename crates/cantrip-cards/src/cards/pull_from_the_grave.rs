use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return up to two target creature cards from your graveyard to your hand. You gain 2 life." The cards that are still
/// legal targets come back together (CR 608.2b); you gain the life whether you chose none or some. If you chose targets and
/// every one is illegal, the spell does not resolve at all (CR 608.2b), and that is the core's business.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::legal_objects(g, item);
    g.return_to_hand(&cards);
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], effect),
    ..oracle::PULL_FROM_THE_GRAVE
};
