use cantrip_api::*;
use cantrip_mechanics::chosen_type;
use cantrip_oracle as oracle;

/// "Choose a creature type. Return all creature cards of the chosen type from your graveyard to the battlefield." The type is
/// chosen as the spell resolves (CR 608.2d); the creature cards in the caster's graveyard that have it, read where they are
/// (a changeling has every type, CR 702.73a), enter together under the caster's control (CR 110.2a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = chosen_type::creature_type_now(g, you, item.object)?;
    let cards: Vec<(ObjectId, PlayerId)> = g
        .state()
        .zone(Zone::graveyard(you))
        .iter()
        .copied()
        .filter(|&card| {
            let chars = g.chars(card);
            chars.is(CardType::Creature) && chars.has_subtype(chosen)
        })
        .map(|card| (card, you))
        .collect();
    g.put_all_onto_battlefield(&cards)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::BLOODLINE_BIDDING };
