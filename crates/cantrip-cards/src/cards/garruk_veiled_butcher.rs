use cantrip_api::*;
use cantrip_mechanics::replacements::OPPONENTS_CREATURES_EXILED_INSTEAD_OF_DYING;
use cantrip_mechanics::{choose_from_hand, each_player_sacrifices, hand, targets, tokens};
use cantrip_oracle as oracle;

/// +2: Up to one target creature gets -4/-1 until your next turn (CR 611.2a).
fn plus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0)
        && g.is_on_battlefield(creature)
    {
        g.add_effect(item.object, creature, EffectKind::Pump(-4, -1), Duration::UntilNextTurnOf(item.controller));
    }
    Ok(())
}

/// −2: Each player sacrifices a creature of their choice. If you sacrificed a creature this way, create a 4/4 green Beast
/// creature token with trample. Each player chooses in turn order (CR 101.4) and the creatures are sacrificed together
/// (CR 701.21a); a player with a single creature has no choice.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let chosen = each_player_sacrifices(g, item.object, 1, |q, id| q.chars(id).is(CardType::Creature))?;
    let yours = chosen.into_iter().find(|&(who, _)| who == you).map(|(_, id)| id);
    if let Some(creature) = yours
        && g.obj(creature).became.is_some()
    {
        tokens::create(g, you, &tokens::BEAST_4_4_TRAMPLE, 1)?;
    }
    Ok(())
}

/// −3: Each opponent discards two cards. For each opponent who didn't discard two nonland cards this way, you draw a
/// card. Each opponent chooses in turn order and the cards are discarded together (CR 101.4, 701.9); an opponent with
/// fewer cards, or whom this ability can't make discard (CR 101.2), is one who didn't discard two nonland cards, and so is one
/// whose card a replacement put into a hidden zone unrevealed: it is undefined, no nonland card (CR 701.9c).
fn minus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| p != you).collect();
    let mut chosen: Vec<ObjectId> = Vec::new();
    for &who in &opponents {
        if !cantrip_mechanics::players::reached(g, who) {
            continue;
        }
        chosen.extend(choose_from_hand(g, who, item.object, 2)?);
    }
    let discarded = g.discard_together(&chosen);
    let short = opponents
        .iter()
        .filter(|&&who| {
            discarded.iter().filter(|&&card| g.obj(card).owner == who && hand::discarded_nonland_card(&*g, card)).count() < 2
        })
        .count();
    for _ in 0..short {
        g.draw(you, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    replacements: &[OPPONENTS_CREATURES_EXILED_INSTEAD_OF_DYING],
    abilities: &[
        ability(loyalty(2), plus_two, "+2: Up to one target creature gets -4/-1 until your next turn.").targets(&[targets::CREATURE.optional()]),
        ability(loyalty(-2), minus_two, "\u{2212}2: Each player sacrifices a creature of their choice. If you sacrificed a creature this way, create a 4/4 green Beast creature token with trample."),
        ability(loyalty(-3), minus_three, "\u{2212}3: Each opponent discards two cards. For each opponent who didn't discard two nonland cards this way, you draw a card."),
    ],
    ..oracle::GARRUK_VEILED_BUTCHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::BEAST_4_4_TRAMPLE];
