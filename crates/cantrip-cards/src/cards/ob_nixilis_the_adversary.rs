use cantrip_api::*;
use cantrip_mechanics::{casualty, choose_from_hand, effects, players, targets, tokens};
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

const DISCARD: Question = Question::new("DiscardInsteadOfLosingLife", "discard a card instead of losing 2 life?", "Otherwise you lose 2 life", "Discard a card", "Lose 2 life");

/// The 1/1 red Devil creature token with "When this token dies, it deals 1 damage to any target."
pub static DEVIL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(THIS_DIES, effects::damage_first_target::<1>, "When this token dies, it deals 1 damage to any target.").targets(&[targets::ANY])],
    text: "When this token dies, it deals 1 damage to any target.",
    ..token!(Devil)
};

/// "The copy isn't legendary and has starting loyalty X" (CR 707.9b, 707.10f): the copy of the spell, a token as it resolves,
/// with X as announced for this spell.
fn copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let loyalty = casualty::x(g, item) as i32;
    let values = g.copiable_values(item.object).nonlegendary().with_loyalty(loyalty);
    g.copy_spell_as(item.object, item.controller, true, values)?;
    Ok(())
}

/// +1: "Each opponent loses 2 life unless they discard a card. If you control a Demon or Devil, you gain 2 life."
/// Each opponent in turn order (CR 101.4) chooses whether to discard (nothing is asked of an empty hand, which cannot),
/// then the discards and the losses of life all happen together, one action (the rulings, CR 101.4, 608.2d; `Game::together`). The Demon or Devil is by
/// the words of this card (CR 612).
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| g.state().opponents(you).any(|o| o == p)).collect();
    let (mut discards, mut losses) = (Vec::new(), Vec::new());
    for opponent in opponents {
        if !players::reached(g, opponent) {
            continue;
        }
        if g.state().zone(Zone::hand(opponent)).is_empty() || !effects::ask_may(g, opponent, item.object, Purpose::Ask(&DISCARD))? {
            losses.push((opponent, -2));
        } else {
            discards.extend(choose_from_hand(g, opponent, item.object, 1)?);
        }
    }
    g.together(Together { discards: &discards, life: &losses, ..Default::default() });
    g.end_sentence();
    let words = g.words(item.object);
    let (demon, devil) = (words.subtype(Subtype::Demon), words.subtype(Subtype::Devil));
    if g.permanents_of(you).into_iter().any(|id| g.chars(id).has_subtype(demon) || g.chars(id).has_subtype(devil)) {
        g.gain_life(you, 2);
    }
    Ok(())
}

/// \-2: "Create a 1/1 red Devil creature token with ..."
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DEVIL, 1)?;
    Ok(())
}

/// \-7: "Target player draws seven cards and loses 7 life."
fn minus_seven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.draw(player, 7);
        g.lose_life(player, 7);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    optional_cost: Some(casualty::X),
    labels: &["Cast without casualty", "Casualty X: sacrifice a creature with power X or greater"],
    abilities: &[
        casualty::trigger_with("Casualty X. The copy isn't legendary and has starting loyalty X.", copy),
        ability(loyalty(1), plus_one, "+1: Each opponent loses 2 life unless they discard a card. If you control a Demon or Devil, you gain 2 life."),
        ability(loyalty(-2), minus_two, "−2: Create a 1/1 red Devil creature token with \"When this token dies, it deals 1 damage to any target.\""),
        ability(loyalty(-7), minus_seven, "−7: Target player draws seven cards and loses 7 life.").targets(&[targets::PLAYER]),
    ],
    ..oracle::OB_NIXILIS_THE_ADVERSARY
};

pub static EXTRA: &[&CardDef] = &[&DEVIL];
