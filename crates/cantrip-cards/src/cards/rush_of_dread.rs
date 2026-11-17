use cantrip_api::*;
use cantrip_mechanics::{choose_from_hand, effects, half_chosen_to_sacrifice, spree, targets};
use cantrip_oracle as oracle;

/// "Target opponent sacrifices half the creatures they control of their choice, rounded up." (CR 701.21a, 107.1a): half of the
/// creatures they control as this resolves, those they can't be made to sacrifice counted among them.
fn sacrifice_half_the_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(opponent).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = half_chosen_to_sacrifice(g, opponent, item.object, creatures, true)?;
    g.sacrifice(opponent, &chosen);
    Ok(())
}

/// "Target opponent discards half the cards in their hand, rounded up." (CR 701.9, 107.1a): half of the hand they have as this
/// resolves, which the earlier modes may have changed (CR 700.2, the ruling), chosen by them and discarded together.
fn discard_half_their_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let half = (g.state().zone(Zone::hand(opponent)).len() as u32 + 1) / 2;
    let chosen = choose_from_hand(g, opponent, item.object, half)?;
    g.discard_together(&chosen);
    Ok(())
}

/// "Target opponent loses half their life, rounded up." (CR 119.3, 107.1a)
fn lose_half_their_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        effects::lose_half_their_life(g, opponent);
    }
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(sacrifice_half_the_creatures, "+ {1} — Target opponent sacrifices half the creatures they control of their choice, rounded up.")
        .targets(&[targets::OPPONENT])
        .cost(mana("{1}")),
    mode(discard_half_their_hand, "+ {2} — Target opponent discards half the cards in their hand, rounded up.")
        .targets(&[targets::OPPONENT])
        .cost(mana("{2}")),
    mode(lose_half_their_life, "+ {2} — Target opponent loses half their life, rounded up.")
        .targets(&[targets::OPPONENT])
        .cost(mana("{2}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::RUSH_OF_DREAD };
