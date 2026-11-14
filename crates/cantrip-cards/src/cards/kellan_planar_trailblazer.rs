use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Exile the top card of your library. You may play that card this turn" (CR 305.1, 601.3).
fn exile_top_play_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_card_play_this_turn(g, item)
}

/// {1}{R}: "If Kellan is a Scout, it becomes a Human Faerie Detective and gains [the trigger]" (CR 205.1a, 613.1d, 611.2a): the
/// check is as the ability resolves; the new creature types replace the old ones, with no end, and the granted ability is the
/// card whose ability is resolving (a copy of Kellan, or an Ooze copying one, grants that card's trigger).
fn become_detective(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let scout = g.words(item.object).subtype(Subtype::Scout);
    if !g.chars(item.object).has_subtype(scout) {
        return Ok(());
    }
    let human = g.words(item.object).subtype(Subtype::Human);
    let faerie = g.words(item.object).subtype(Subtype::Faerie);
    let detective = g.words(item.object).subtype(Subtype::Detective);
    let card = item.ability.card;
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetCreatureType(human), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(faerie), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(detective), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(card, 0)), Duration::Indefinite);
    });
    Ok(())
}

/// {2}{R}: "If Kellan is a Detective, it becomes a 3/2 Human Faerie Rogue and gains double strike" (CR 205.1a, 613.4b): the creature
/// types and the base power and toughness replace the old ones; the trigger it gained stays (nothing removes it).
fn become_rogue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let detective = g.words(item.object).subtype(Subtype::Detective);
    if !g.chars(item.object).has_subtype(detective) {
        return Ok(());
    }
    let human = g.words(item.object).subtype(Subtype::Human);
    let faerie = g.words(item.object).subtype(Subtype::Faerie);
    let rogue = g.words(item.object).subtype(Subtype::Rogue);
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetPt(3, 2), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::SetCreatureType(human), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(faerie), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(rogue), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[Keyword::DoubleStrike])), Duration::Indefinite);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{R}"),
            become_detective,
            "{1}{R}: If Kellan is a Scout, it becomes a Human Faerie Detective and gains \"Whenever Kellan deals combat damage to a player, exile the top card of your library. You may play that card this turn.\"",
        ),
        ability(cost("{2}{R}"), become_rogue, "{2}{R}: If Kellan is a Detective, it becomes a 3/2 Human Faerie Rogue and gains double strike."),
    ],
    delayed: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        exile_top_play_this_turn,
        "Whenever Kellan deals combat damage to a player, exile the top card of your library. You may play that card this turn.",
    )],
    ..oracle::KELLAN_PLANAR_TRAILBLAZER
};
