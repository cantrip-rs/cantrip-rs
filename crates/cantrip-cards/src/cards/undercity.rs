use cantrip_api::*;
use cantrip_mechanics::tokens::{self, TREASURE};
use cantrip_mechanics::library::{Find, search_to_hand};
use cantrip_mechanics::{choose, effects, goad, scry, targets};

// The Undercity dungeon (CR 309): nine rooms, the arrows between them, and each room's ability, which triggers as a
// venture marker moves into it (CR 309.4c). The initiative ventures into it (CR 726.2); the rules (`venture`,
// CR 701.49) walk the marker.

/// Room 0, Secret Entrance.
fn secret_entrance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_hand(g, item.controller, item.object, Find::ONE.revealed(), |q, c| {
        let chars = q.chars(c);
        chars.is(CardType::Land) && chars.supertypes.basic
    })?;
    Ok(())
}

/// Room 1, Forge.
fn forge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 2);
    }
    Ok(())
}

/// Room 2, Lost Well.
fn lost_well(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    scry(g, item.controller, item.object, 2)
}

/// Room 3, Trap!
fn trap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Player(p) = item.target(0)
    {
        g.lose_life(p, 5);
    }
    Ok(())
}

/// Room 4, Arena: goad (CR 701.15a), until the controller's next turn.
fn arena(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        goad::goad(g, item.object, creature, Duration::UntilNextTurnOf(item.controller));
    }
    Ok(())
}

/// Room 5, Stash.
fn stash(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)
}

/// A 4/1 black Skeleton creature token with menace.
pub static SKELETON: CardDef = CardDef {
    name: "Skeleton",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Skeleton]),
    power: Some(4),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Menace]),
    text: "Menace",
    ..CardDef::BLANK
};

/// Room 7, Catacombs.
fn catacombs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SKELETON, 1)?;
    Ok(())
}

/// Room 8, Throne of the Dead Three: reveal the top ten cards, put a creature card from among them onto the
/// battlefield with three +1/+1 counters, hexproof until your next turn, then shuffle.
fn throne(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let revealed: Vec<ObjectId> = g.state().zone(Zone::library(player)).iter().take(10).copied().collect();
    if !revealed.is_empty() {
        g.reveal(player, &revealed);
        let eligible: Vec<ObjectId> = revealed.iter().copied().filter(|&c| g.chars(c).is(CardType::Creature)).collect();
        if let Some(card) = choose::card_from(g, player, item.object, Purpose::PutOntoBattlefield, revealed, eligible)? {
            let creature = g.put_onto_battlefield(card, player, false)?;
            if g.is_on_battlefield(creature) {
                g.add_counters(creature, CounterKind::PlusOne, 3);
                g.add_effect(
                    item.object,
                    creature,
                    EffectKind::AddKeywords(Keywords::of(&[Keyword::Hexproof])),
                    Duration::UntilNextTurnOf(player),
                );
            }
        }
    }
    g.shuffle(player);
    Ok(())
}

const fn room(text: &'static str, targets: &'static [TargetSpec], effect: Effect) -> AbilityDef {
    delayed_trigger(effect, text)
        .targets(targets)
}

pub static CARD: CardDef = CardDef {
    name: "Undercity",
    has_cost: false,
    dungeon: &[&[1, 2], &[3, 4], &[4, 5], &[6], &[6, 7], &[7], &[8], &[8], &[]],
    abilities: &[
        room("Search your library for a basic land card, reveal it, put it into your hand, then shuffle.", &[], secret_entrance),
        room("Put two +1/+1 counters on target creature.", &[targets::CREATURE], forge),
        room("Scry 2.", &[], lost_well),
        room("Target player loses 5 life.", &[targets::PLAYER], trap),
        room("Goad target creature.", &[targets::CREATURE], arena),
        room("Create a Treasure token.", &[], stash),
        room("Draw a card.", &[], |g, item| effects::draw(g, item, 1)),
        room("Create a 4/1 black Skeleton creature token with menace.", &[], catacombs),
        room(
            "Reveal the top ten cards of your library. Put a creature card from among them onto the battlefield with three +1/+1 counters on it. It gains hexproof until your next turn. Then shuffle.",
            &[],
            throne,
        ),
    ],
    labels: &[
        "Secret Entrance \u{2014} Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        "Forge \u{2014} Put two +1/+1 counters on target creature.",
        "Lost Well \u{2014} Scry 2.",
        "Trap! \u{2014} Target player loses 5 life.",
        "Arena \u{2014} Goad target creature.",
        "Stash \u{2014} Create a Treasure token.",
        "Archives \u{2014} Draw a card.",
        "Catacombs \u{2014} Create a 4/1 black Skeleton creature token with menace.",
        "Throne of the Dead Three \u{2014} Reveal the top ten cards of your library. Put a creature card from among them onto the battlefield with three +1/+1 counters on it. It gains hexproof until your next turn. Then shuffle.",
    ],
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&SKELETON, &TREASURE];
