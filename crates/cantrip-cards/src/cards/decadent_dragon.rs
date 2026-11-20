use cantrip_api::*;
use cantrip_mechanics::library::exile_top_face_down_may_look_and_play;
use cantrip_mechanics::{targets, tokens, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Exile the top two cards of target opponent's library face down. You may look at and play those cards for as long as they
/// remain exiled" (CR 406.3, 601.3, 305.1): the cards stay in their owner's exile, and the permission, to cast them or play a land
/// among them, lasts until they leave it. No mana of another type may be spent. Nothing happens if the target is no longer legal
/// (CR 608.2b).
fn exile_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let grant = PlayGrant { lands: true, ..PlayGrant::CAST };
    exile_top_face_down_may_look_and_play(g, opponent, 2, item.controller, grant);
    Ok(())
}

/// The Adventure (CR 715).
pub static EXPENSIVE_TASTE: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], exile_two),
    ..oracle::EXPENSIVE_TASTE
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, tokens::create_a_treasure, "Whenever this creature attacks, create a Treasure token.")],
    adventure: Some(&EXPENSIVE_TASTE),
    ..oracle::DECADENT_DRAGON
};

pub static EXTRA: &[&CardDef] = &[&EXPENSIVE_TASTE, &tokens::TREASURE];
