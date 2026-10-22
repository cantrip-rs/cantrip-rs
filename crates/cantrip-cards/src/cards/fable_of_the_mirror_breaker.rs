use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{TREASURE, create_a_treasure};
use cantrip_mechanics::exile_and_return_transformed;
use cantrip_oracle as oracle;

/// The 2/2 red Goblin Shaman token chapter I makes.
pub static GOBLIN_SHAMAN: CardDef = CardDef {
    name: "Goblin Shaman Token",
    token_name: Some(&[Subtype::Goblin, Subtype::Shaman]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Goblin, Subtype::Shaman]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(cantrip_mechanics::triggers::THIS_ATTACKS, create_a_treasure, "Whenever this token attacks, create a Treasure token.")],
    text: "Whenever this token attacks, create a Treasure token.",
    ..CardDef::BLANK
};

fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &GOBLIN_SHAMAN, 1)?;
    Ok(())
}

/// "You may discard up to two cards. If you do, draw that many cards." Asked whenever there is a card to discard;
/// choosing none discards and draws nothing.
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let chosen = choose::cards(g, player, item.object, Purpose::Discard, hand, 2, true)?;
    g.discard_together(&chosen);
    if !chosen.is_empty() {
        g.draw(player, chosen.len() as u32);
    }
    Ok(())
}

fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

// --- Reflection of Kiki-Jiki ------------------------------------------------------------------------------------

/// "Another target nonlegendary creature you control."
fn another_nonlegendary_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if id != asker.source && targets::permanent(q, id) && q.controller(id) == asker.controller && {
        let c = q.chars(id);
        c.is(CardType::Creature) && !c.supertypes.legendary
    })
}

/// "Create a token that's a copy of another target nonlegendary creature you control, except it has haste. Sacrifice
/// it at the beginning of the next end step."
fn copy_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = g.card_id(&BACK);
    cantrip_mechanics::copy::create_hasty_token_copy_sacrificed_at_end_step(g, item, me, 0)
}

pub static BACK: CardDef = CardDef {
    abilities: &[ability(cost("{1}, {T}"), copy_creature, "{1}, {T}: Create a token that's a copy of another target nonlegendary creature you control, except it has haste. Sacrifice it at the beginning of the next end step.")
        .targets(&[target(another_nonlegendary_creature_you_control)])],
    delayed: &[cantrip_mechanics::copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::REFLECTION_OF_KIKI_JIKI
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Create a 2/2 red Goblin Shaman creature token with \"Whenever this token attacks, create a Treasure token.\""),
        chapter(2, chapter_two, "II \u{2014} You may discard up to two cards. If you do, draw that many cards."),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    back: Some(&BACK),
    ..oracle::FABLE_OF_THE_MIRROR_BREAKER
};

pub static EXTRA: &[&CardDef] = &[&GOBLIN_SHAMAN, &TREASURE];
