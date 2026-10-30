use cantrip_api::*;
use cantrip_mechanics::library::exile_top_play_until_end_of_next_turn;
use cantrip_oracle as oracle;

/// "Exile the top two cards of your library. Until the end of your next turn, you may play those cards."
/// The permission lasts through the controller's next turn, including lands under normal land-play timing (CR 305.1).
fn come_in_peace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_top_play_until_end_of_next_turn(g, item.controller, 2);
    Ok(())
}

/// "Encounter the Unknown deals 2 damage to each creature and each player." One simultaneous damage event includes both
/// players and every creature on the battlefield (CR 608.2c, 120.2).
fn shoot_to_kill(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hits: Vec<(ObjectId, Target, u16)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (item.object, Target::Object(id), 2))
        .collect();
    hits.extend(g.state().apnap().map(|p| (item.object, Target::Player(p), 2)));
    g.damage_together(&hits);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(come_in_peace, "Come in Peace — Exile the top two cards of your library. Until the end of your next turn, you may play those cards."),
    mode(shoot_to_kill, "Shoot to Kill — Encounter the Unknown deals 2 damage to each creature and each player."),
])
.choose(1, 1);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::ENCOUNTER_THE_UNKNOWN
};
