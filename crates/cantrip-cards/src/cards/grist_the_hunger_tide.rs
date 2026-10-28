use cantrip_api::*;
use cantrip_mechanics::{effects, select};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The 1/1 black and green Insect creature token the +1 makes.
pub static INSECT: CardDef = CardDef {
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Insect)
};

/// "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types": a
/// characteristic-defining ability (CR 604.3) that works in every zone, in two layers (CR 613.1d, 613.4a).
fn insect_types(q: &dyn LayerQuery, grist: ObjectId, chars: &mut Chars) {
    if !q.state().obj(grist).zone.is(ZoneKind::Battlefield) {
        chars.types = chars.types.with(CardType::Creature);
        chars.subtypes = chars.subtypes.with(q.words(grist).subtype(Subtype::Insect));
    }
}

fn insect_size(q: &dyn LayerQuery, grist: ObjectId, chars: &mut Chars) {
    if !q.state().obj(grist).zone.is(ZoneKind::Battlefield) {
        chars.power = Some(1);
        chars.toughness = Some(1);
    }
}

/// +1: "Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way,
/// put a loyalty counter on Grist and repeat this process." Grist herself, milled, is an Insect card. The counter goes only on Grist if
/// she is still the permanent that activated this; otherwise the ability goes on making tokens and milling.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let grist = item.object;
    let insect = g.words(grist).subtype(Subtype::Insect);
    loop {
        cantrip_mechanics::tokens::create(g, player, &INSECT, 1)?;
        let Some(&milled) = g.mill(player, 1).first() else { return Ok(()) };
        if !g.chars(milled).has_subtype(insect) {
            return Ok(());
        }
        if targets::permanent(g, grist) {
            g.add_counters(grist, CounterKind::Loyalty, 1);
        }
    }
}

/// -2: "You may sacrifice a creature. When you do, destroy target creature or planeswalker." The creature is chosen
/// as the ability resolves, not targeted, and the reflexive trigger exists only if one was sacrificed (CR 603.12),
/// whether or not Grist is still on the battlefield.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    if effects::may_sacrifice(g, player, item.object, select::creature_cards(select::up_to(1)))?.is_empty() {
        return Ok(());
    }
    let me = g.card_id(&CARD);
    g.reflexive_trigger(item.object, player, AbilityRef::delayed(me, 0), Target::None);
    Ok(())
}

fn destroy_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::destroy_target(g, item, 0);
    Ok(())
}

/// -5: "Each opponent loses life equal to the number of creature cards in your graveyard", counted as it resolves.
fn minus_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let count = cantrip_mechanics::creature_cards_in_graveyard(g, player);
    if count > 0 {
        g.each_opponent_loses_life(player, count as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process."),
        ability(loyalty(-2), minus_two, "\u{2212}2: You may sacrifice a creature. When you do, destroy target creature or planeswalker."),
        ability(loyalty(-5), minus_five, "\u{2212}5: Each opponent loses life equal to the number of creature cards in your graveyard."),
    ],
    delayed: &[delayed_trigger(destroy_target, "When you do, destroy target creature or planeswalker.")
        .targets(&[targets::CREATURE_OR_PLANESWALKER])],
    statics: &[
        characteristic_defining(Layer::Type, insect_types, "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types."),
        characteristic_defining(Layer::PtDefine, insect_size, "As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types."),
    ],
    ..oracle::GRIST_THE_HUNGER_TIDE
};

pub static EXTRA: &[&CardDef] = &[&INSECT];
