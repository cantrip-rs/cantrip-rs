use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "If you don't control a planeswalker" (CR 603.4): an intervening "if", judged when the end step begins and again as the
/// ability resolves, on the planeswalkers its controller controls now.
fn you_dont_control_a_planeswalker(q: &dyn Query, asker: &Asker) -> bool {
    !q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).is(CardType::Planeswalker))
}

/// The ability each token copy has by the copy's exception (CR 707.9a). Not a card anyone plays: a definition for the
/// exception's ability to be referred to by. It is part of the copy's copiable values (CR 707.9b), so a copy of a token
/// Face Yourself made has it too.
pub static EXCEPTION: CardDef = CardDef {
    name: "Face Yourself (copy exception)",
    abilities: &[triggered(
        triggers::EACH_END_STEP,
        effects::sacrifice_self,
        "At the beginning of the end step, if you don't control a planeswalker, sacrifice this creature.",
    )
    .condition(you_dont_control_a_planeswalker)],
    ..CardDef::BLANK
};

/// "For each creature target player controls, create a token that's a copy of that creature, except it has haste and
/// [the exception's ability]" (CR 707.2, 707.9a, 111.1): the creatures that player controls as the spell resolves, each
/// copied as it is (a token's original characteristics, a face-down creature's face-down values, CR 707.2), the tokens
/// created together so each sees the others enter (the ruling, CR 101.4), under the spell's controller. Nothing if the
/// player is no longer a legal target (CR 608.2b).
fn copy_each_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let exception = AbilityRef { card: g.card_id(&EXCEPTION), index: 0 };
    let copies: Vec<(Copiable, PlayerId)> = g
        .permanents_of(player)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (g.copiable_values(id).with_keyword(Keyword::Haste).with_ability(exception), item.controller))
        .collect();
    g.create_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::PLAYER], copy_each_creature), ..oracle::FACE_YOURSELF };

pub static EXTRA: &[&CardDef] = &[&EXCEPTION];
