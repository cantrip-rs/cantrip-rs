use cantrip_api::*;
use cantrip_mechanics::{damage_target, instant_sorcery_or_adventure_cards_in_graveyard, targets};
use cantrip_oracle as oracle;

/// "Frantic Firebolt deals X damage to target creature, where X is 2 plus the number of cards in your graveyard that are instant
/// cards, sorcery cards, and/or have an Adventure." (CR 120.3, 715.1): counted as it resolves, and the spell itself is still on the
/// stack (CR 608.2n), so it is not counted. Nothing happens if the creature is no longer a legal target (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = instant_sorcery_or_adventure_cards_in_graveyard(g, item.controller) as u32;
    damage_target(g, item, 0, 2 + cards);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::FRANTIC_FIREBOLT };
