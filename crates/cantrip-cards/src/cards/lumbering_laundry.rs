use cantrip_api::*;
use cantrip_mechanics::disguise;
use cantrip_oracle as oracle;

/// "Face-down creatures you don't control" (CR 708.5): any other player's, asked of each face-down permanent as it is now, so
/// one turned face down later in the turn is seen too (the ruling).
fn creature_you_dont_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.state().obj(id).controller != asker.controller && q.chars(id).is(CardType::Creature)
}

/// "Until end of turn, you may look at face-down creatures you don't control any time." (CR 611.2a, 708.5): the player is
/// given the play rule for the turn, as an effect of the resolved ability. It is the player's, so it outlasts the Laundry
/// leaving the battlefield; once the turn ends they can no longer look (the ruling).
fn look_at_face_down_creatures_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.ability.card;
    instruction(g, |g| g.add_effect(item.object, item.object, EffectKind::PlayerPlayRule(AbilityRef::delayed(card, 0)), Duration::EndOfTurn));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::mana(mana("{2}")),
        look_at_face_down_creatures_until_end_of_turn,
        "{2}: Until end of turn, you may look at face-down creatures you don't control any time.",
    )],
    delayed: &[granted_play_rule(
        PlayRule::LookAtFaceDown(creature_you_dont_control),
        "You may look at face-down creatures you don't control any time.",
    )],
    ways: &[disguise::way(Cost::mana(mana("{5}")), "Disguise {5}")],
    ..oracle::LUMBERING_LAUNDRY
};
