use cantrip_api::*;
use cantrip_mechanics::{hand::choose_from_revealed_hand_of_kind, targets, threshold, triggers::THIS_ENTERS_IF_CAST};
use cantrip_oracle as oracle;

/// "When this creature enters, if you cast it, target opponent reveals their hand. You choose a nonland card from it. Exile
/// that card." An intervening "if" (CR 603.4) the history answers: a creature put onto the battlefield, or a token copy of it,
/// was not cast. The card is exiled with this object (CR 607.2a) for the threshold ability to find: linked to the creature as
/// it was when the trigger was put on the stack, so a Summoner that has left (and maybe come back, a new object, CR 400.7)
/// before this resolves still exiles the card, but no Summoner's ability can cast it. Looking at the hand is part of the
/// ability, so it is revealed even with nothing eligible in it (`choose_from_revealed_hand_of_kind`).
fn exile_a_nonland_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    let chosen = choose_from_revealed_hand_of_kind(g, item, opponent, Purpose::Exile, "nonland", |q, c| !q.chars(c).is(CardType::Land))?;
    g.exile_with(&chosen, g.linked(item), false);
    Ok(())
}

/// "Threshold — As long as there are seven or more cards in your graveyard, you may cast the exiled card": a static ability
/// (CR 604.1, 611.3a) that lets this permanent's controller ("you", CR 109.5) cast the card the linked enters ability exiled
/// (CR 607.2a) while the condition holds, asked whenever the cast would begin (CR 601.3); threshold is an ability word with no
/// rules meaning of its own (CR 207.2c). It is a permission, so the card is cast at the time its type allows, for its mana
/// cost or an alternative cost it has. It functions only while this permanent is on the battlefield with its abilities.
fn may_cast_the_exiled_card(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && threshold(q, player) && q.exiled_with(q.linked_to(asker)).contains(&card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_IF_CAST,
        exile_a_nonland_card,
        "When this creature enters, if you cast it, target opponent reveals their hand. You choose a nonland card from it. Exile that card.",
    )
    .targets(&[targets::OPPONENT])],
    // "... and mana of any type can be spent to cast that spell" (CR 609.4b).
    exile_casts: &[ExileCast { text: "Threshold: you may cast this with mana of any type", allows: may_cast_the_exiled_card, additional: Cost::NONE, any_type: true, lands: false, life_for_mana_value: false, instead: None }],
    ..oracle::NULL_SUMMONER
};
