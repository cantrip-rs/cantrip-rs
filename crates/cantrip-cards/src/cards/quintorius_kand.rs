use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_FROM_EXILE;
use cantrip_mechanics::{discover, effects, targets, tokens};
use cantrip_oracle as oracle;

/// The 3/2 red and white Spirit creature token its +1 makes.
pub static SPIRIT: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::White).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(2),
    ..token!(Spirit)
};

/// "Whenever you cast a spell from exile, Quintorius Kand deals 2 damage to each opponent and you gain 2 life." Both happen
/// even if Quintorius has left the battlefield by the time it resolves (CR 113.7a, 608.2h).
fn ping_and_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 2)
}

/// "+1: Create a 3/2 red and white Spirit creature token."
fn spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SPIRIT, 1)?;
    Ok(())
}

/// "-3: Discover 4" (CR 701.57a).
fn discover_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 4)
}

/// "-6: Exile any number of target cards from your graveyard. Add {R} for each card exiled this way. You may play those
/// cards this turn." The targets still legal are exiled together (CR 608.2b); the mana is one {R} for each that really reached
/// exile (CR 406.3), and each of those may be played this turn, a land as a land play (CR 305.1, 601.3).
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = (0..item.target_count as usize).filter_map(|slot| targets::object(g, item, slot)).collect();
    let exiled: Vec<ObjectId> = g.exile_all(&cards).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    g.add_mana(item.controller, Mana::R, exiled.len() as u32);
    for card in exiled {
        g.allow_play(card, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_CAST_A_SPELL_FROM_EXILE, ping_and_gain, "Whenever you cast a spell from exile, Quintorius Kand deals 2 damage to each opponent and you gain 2 life."),
        ability(loyalty(1), spirit, "+1: Create a 3/2 red and white Spirit creature token."),
        ability(loyalty(-3), discover_four, "\u{2212}3: Discover 4."),
        ability(loyalty(-6), exile_and_play, "\u{2212}6: Exile any number of target cards from your graveyard. Add {R} for each card exiled this way. You may play those cards this turn.")
            .targets(&[up_to(TargetSpec::ANY_NUMBER, targets::any_card_in_your_graveyard)]),
    ],
    ..oracle::QUINTORIUS_KAND
};

pub static EXTRA: &[&CardDef] = &[&SPIRIT];
