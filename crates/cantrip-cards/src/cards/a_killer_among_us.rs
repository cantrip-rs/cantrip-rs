use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, text_change, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 white Human, 1/1 blue Merfolk and 1/1 red Goblin creature token. Then secretly choose Human, Merfolk, or
/// Goblin" (CR 111.1, 608.2c, 607.2d): the three tokens are created at once, and then the controller chooses, in private: no
/// other player is shown the type, in the view of this enchantment or in what they are asked, until it is revealed as a cost.
/// It is kept for this enchantment's text and for the player who chose it, so each copy of the enchantment keeps its own.
fn create_tokens_and_secretly_choose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let created: Vec<(CardId, PlayerId)> = [&tokens::HUMAN, &tokens::MERFOLK, &tokens::GOBLIN].iter().map(|&def| (g.card_id(def), you)).collect();
    g.create_tokens_worded(&created, false, None, g.words(item.object))?;
    let words = g.words(item.object);
    let choices = vec![words.subtype(Subtype::Human), words.subtype(Subtype::Merfolk), words.subtype(Subtype::Goblin)];
    let chosen = text_change::one_subtype(g, you, item.object, choices, WordUse::Chosen)?;
    let by = g.linked(item);
    g.remember_secret_subtype(by, chosen, you);
    Ok(())
}

/// "If target attacking creature token is the chosen type, put three +1/+1 counters on it and it gains deathtouch until end of
/// turn" (CR 608.2b, 608.2h): the type is the one revealed as the cost was paid, kept on this ability (`announced_type`) since
/// the enchantment was sacrificed to pay for it; the target is checked as it resolves, a token of another type gets nothing.
fn put_counters_and_deathtouch_if_the_chosen_type(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(token), Some(chosen)) = (targets::object(g, item, 0), item.announced_type()) else { return Ok(()) };
    if !g.chars(token).has_subtype(chosen) {
        return Ok(());
    }
    g.add_counters(token, CounterKind::PlusOne, 3);
    effects::gain_until_end_of_turn(g, item.object, token, Keyword::Deathtouch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            create_tokens_and_secretly_choose,
            "When this enchantment enters, create a 1/1 white Human creature token, a 1/1 blue Merfolk creature token, and a 1/1 red Goblin creature token. Then secretly choose Human, Merfolk, or Goblin.",
        ),
        ability(
            SACRIFICE_THIS.and(Cost { reveal_secret_type: true, ..Cost::NONE }),
            put_counters_and_deathtouch_if_the_chosen_type,
            "Sacrifice this enchantment, Reveal the creature type you chose: If target attacking creature token is the chosen type, put three +1/+1 counters on it and it gains deathtouch until end of turn.",
        )
        .targets(&[targets::ATTACKING_CREATURE_TOKEN]),
    ],
    ..oracle::A_KILLER_AMONG_US
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN, &tokens::MERFOLK, &tokens::GOBLIN];
