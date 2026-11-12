//! CR 305.7, an effect that sets a land's subtype, on a table of test-only cards for the paths no printed card in the engine
//! reaches. The rest are scenarios of printed cards (Magus of the Moon with Mutavault, Azorius Chancery, Urborg and
//! Multiversal Passage). These are:
//!
//! - a land's printed rules that live on its card rather than among its abilities - a replacement effect - and a printed
//!   keyword, which it loses, beside a keyword another effect granted it, which it keeps;
//! - a land whose own static ability sets other lands' subtypes, which another such ability takes away whatever their
//!   timestamps (CR 613.8a): no printed land the engine has sets other lands' subtypes;
//! - the same done by a resolved spell's effect (`EffectKind::SetLandType`, "target land becomes a Swamp"): the printed rules
//!   lost, the granted keyword kept, the supertypes unchanged, and a land's ability that sets other lands' subtypes taken away
//!   though the effect is newer (CR 613.8a).

use std::collections::VecDeque;
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::replacements::GAIN_TWICE_THAT_MUCH_LIFE;

/// A basic land to fill hands and libraries: no setter here touches it.
static FILLER: CardDef = CardDef {
    name: "Filler",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    ..CardDef::BLANK
};

/// Another basic land, to be played.
static BASIC: CardDef = CardDef {
    name: "Probe Basic",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    ..CardDef::BLANK
};

fn nonbasic_land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Land) && !c.supertypes.basic
}

fn mountain(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.set_land_subtypes(SubtypeSet::NONE.with(Subtype::Mountain));
}

/// A zero-cost 1/1 creature: "Nonbasic lands are Mountains."
static MOON: CardDef = CardDef {
    name: "Probe Moon",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    statics: &[sets_land_subtypes(nonbasic_land, mountain, "Nonbasic lands are Mountains.")],
    ..CardDef::BLANK
};

/// A nonbasic land: hexproof, "{T}: Add {C}", and "If you would gain life, you gain twice that much life instead."
static LAND: CardDef = CardDef {
    name: "Probe Land",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    keywords: Keywords::of(&[Keyword::Hexproof]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    replacements: &[GAIN_TWICE_THAT_MUCH_LIFE],
    ..CardDef::BLANK
};

fn other_land(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
    source != id && c.is(CardType::Land)
}

fn island(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.set_land_subtypes(SubtypeSet::NONE.with(Subtype::Island));
}

/// A nonbasic land: "Each other land is an Island."
static SEA: CardDef = CardDef {
    name: "Probe Sea",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    statics: &[sets_land_subtypes(other_land, island, "Each other land is an Island.")],
    ..CardDef::BLANK
};

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost instant: "You gain 1 life."
static GAIN: CardDef = CardDef { name: "Probe Gain", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], gain_one), ..CardDef::BLANK };

fn a_land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Land))
}

fn wings(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])), Duration::EndOfTurn);
    }
    Ok(())
}

/// A zero-cost instant: "Target land gains flying until end of turn."
static WINGS: CardDef = CardDef { name: "Probe Wings", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(a_land)], wings), ..CardDef::BLANK };

fn swamp(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::SetLandType(Subtype::Swamp), Duration::EndOfTurn);
    }
    Ok(())
}

/// A zero-cost instant: "Target land becomes a Swamp until end of turn." A resolved effect that sets a land's subtype
/// (`EffectKind::SetLandType`; Cyclopean Tomb's, Shimmering Mirage's).
static SWAMP: CardDef = CardDef { name: "Probe Swamp", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(a_land)], swamp), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&FILLER, &BASIC, &MOON, &LAND, &SEA, &GAIN, &WINGS, &SWAMP]))
}

/// A two-player game: each seat's hand is the named cards, then basic lands.
fn deal(first: &[&str], second: &[&str]) -> Game {
    let list = |names: &[&str]| -> Vec<CardId> {
        let mut deck: Vec<CardId> = names.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect();
        let filler = table().find("Filler").unwrap();
        deck.extend(std::iter::repeat_n(filler, 30 - deck.len()));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![list(first), list(second)], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game.run();
    game
}

/// Plays on: each seat plays or casts the named cards in order whenever it has priority and can, takes the candidate named
/// for a target, and passes otherwise. Stops when `until` holds at a question.
fn play(game: &mut Game, plays: [&[(&str, &str)]; 2], until: impl Fn(&Game) -> bool) {
    let mut plan: [VecDeque<(&str, &str)>; 2] = [plays[0].iter().copied().collect(), plays[1].iter().copied().collect()];
    for _ in 0..200 {
        game.run();
        if until(game) {
            return;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let seat = request.player.0 as usize;
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let wanted = plan[seat].front().copied();
                let option = wanted.and_then(|(name, _)| {
                    options.iter().find(|o| matches!(o.kind, ActionKind::CastSpell | ActionKind::PlayLand) && o.source.is_some_and(|s| game.name(s) == name))
                });
                match option {
                    Some(option) => {
                        let (_, aimed) = plan[seat].pop_front().unwrap();
                        let mut action = Action::of(option);
                        if option.targets_required > 0 {
                            let found = option.target_candidates.iter().copied().find(|c| matches!(c, Target::Object(id) if game.name(*id) == aimed));
                            action.targets = vec![found.unwrap_or_else(|| panic!("no candidate named {aimed}"))];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game did not reach the position");
}

fn on_battlefield(game: &Game, name: &str) -> Option<ObjectId> {
    game.state.battlefield().iter().copied().find(|&id| game.name(id) == name)
}

fn settled(game: &Game, name: &str) -> bool {
    on_battlefield(game, name).is_some() && game.state.stack.is_empty()
}

/// CR 305.7: "It loses all abilities generated from its rules text ... Note that this doesn't remove any abilities that
/// were granted to the land by other effects." Before Probe Moon, Probe Land's replacement doubles a gain of 1 life (the
/// control: 20 to 22). After it, the land is a Mountain with "{T}: Add {R}" alone (CR 305.6), no hexproof and no
/// replacement effect - a gain of 1 is 1 - while the flying an instant granted it stays.
#[test]
fn a_land_whose_subtype_is_set_loses_its_printed_rules_and_keeps_what_was_granted() {
    let mut game = deal(&["Probe Land", "Probe Gain", "Probe Wings", "Probe Moon", "Probe Gain"], &[]);
    play(&mut game, [&[("Probe Land", ""), ("Probe Gain", "")], &[]], |g| g.state.player(PlayerId(0)).life == 22 && g.state.stack.is_empty());
    let land = on_battlefield(&game, "Probe Land").unwrap();
    assert!(!game.lost_all_abilities(land));
    assert!(game.chars(land).has(Keyword::Hexproof));

    play(&mut game, [&[("Probe Wings", "Probe Land"), ("Probe Moon", "")], &[]], |g| settled(g, "Probe Moon"));
    let chars = game.chars(land);
    assert_eq!(chars.subtypes, SubtypeSet::NONE.with(Subtype::Mountain));
    assert!(chars.has(Keyword::Flying), "a keyword another effect granted stays");
    assert!(!chars.has(Keyword::Hexproof), "a printed keyword is gone");
    let abilities: Vec<AbilityRef> = chars.abilities.iter().flatten().copied().collect();
    assert_eq!(abilities, vec![AbilityRef { card: table().intrinsic, index: Mana::R as u8 }], "only the Mountain's {{T}}: Add {{R}}");
    assert!(game.lost_all_abilities(land), "every rule its card prints is gone with its rules text");

    play(&mut game, [&[("Probe Gain", "")], &[]], |g| g.state.player(PlayerId(0)).life == 23 && g.state.stack.is_empty());
    assert_eq!(game.state.player(PlayerId(0)).life, 23);
}

/// Each other land is an Island (Probe Sea), and nonbasic lands are Mountains (Probe Moon). Probe Sea is a nonbasic land,
/// so applying Probe Moon's effect takes Probe Sea's ability away: Probe Sea's effect depends on it (CR 613.8a), Probe
/// Moon's applies first and Probe Sea's no longer exists, whichever is older. The basic land is no Island; Probe Sea is a
/// Mountain. Probe Moon's own source is a creature, which Probe Sea's ability does not touch.
#[test]
fn a_land_that_sets_other_lands_subtypes_loses_that_ability_to_another_such_effect_in_either_order() {
    for moon_first in [false, true] {
        let first: &[(&str, &str)] = if moon_first { &[("Probe Moon", ""), ("Probe Sea", "")] } else { &[("Probe Sea", ""), ("Probe Moon", "")] };
        let mut game = deal(&["Probe Sea", "Probe Moon"], &["Probe Basic"]);
        // The control: with Probe Sea alone, the basic land the opponent plays is an Island.
        if !moon_first {
            play(&mut game, [&first[..1], &[("Probe Basic", "")]], |g| settled(g, "Probe Basic"));
            let basic = on_battlefield(&game, "Probe Basic").unwrap();
            assert_eq!(game.chars(basic).subtypes, SubtypeSet::NONE.with(Subtype::Island));
            play(&mut game, [&first[1..], &[]], |g| settled(g, "Probe Moon"));
        } else {
            play(&mut game, [first, &[("Probe Basic", "")]], |g| settled(g, "Probe Basic") && on_battlefield(g, "Probe Sea").is_some());
        }
        let basic = on_battlefield(&game, "Probe Basic").unwrap();
        let sea = on_battlefield(&game, "Probe Sea").unwrap();
        assert_eq!(game.chars(basic).subtypes, SubtypeSet::NONE, "moon first: {moon_first}");
        assert_eq!(game.chars(sea).subtypes, SubtypeSet::NONE.with(Subtype::Mountain), "moon first: {moon_first}");
        assert!(game.lost_all_abilities(sea));
        assert!(!game.lost_all_abilities(basic));
    }
}

/// CR 305.7 by a resolved effect: Probe Land, made a Swamp by Probe Swamp, has the Swamp's "{T}: Add {B}" alone (CR 305.6), no
/// hexproof and no replacement effect - a gain of 1 life is 1 - while the flying Probe Wings granted it stays. Its card type
/// and its (lack of) supertypes are unchanged, and the effect ends with the turn: the land is itself again.
#[test]
fn a_land_a_resolved_effect_makes_a_swamp_loses_its_printed_rules_until_the_effect_ends() {
    let mut game = deal(&["Probe Land", "Probe Wings", "Probe Swamp", "Probe Gain"], &[]);
    play(&mut game, [&[("Probe Land", ""), ("Probe Wings", "Probe Land"), ("Probe Swamp", "Probe Land")], &[]], |g| {
        g.state.stack.is_empty() && g.state.effects.iter().any(|e| matches!(e.kind, EffectKind::SetLandType(_)))
    });
    let land = on_battlefield(&game, "Probe Land").unwrap();
    let chars = game.chars(land);
    assert_eq!(chars.subtypes, SubtypeSet::NONE.with(Subtype::Swamp));
    assert_eq!((chars.types, chars.supertypes), (TypeSet::of(&[CardType::Land]), Supertypes::default()));
    assert!(chars.has(Keyword::Flying), "a keyword another effect granted stays");
    assert!(!chars.has(Keyword::Hexproof), "a printed keyword is gone");
    let abilities: Vec<AbilityRef> = chars.abilities.iter().flatten().copied().collect();
    assert_eq!(abilities, vec![AbilityRef { card: table().intrinsic, index: Mana::B as u8 }], "only the Swamp's {{T}}: Add {{B}}");
    assert!(game.lost_all_abilities(land), "every rule its card prints is gone with its rules text");

    play(&mut game, [&[("Probe Gain", "")], &[]], |g| g.state.player(PlayerId(0)).life == 21 && g.state.stack.is_empty());
    assert_eq!(game.state.player(PlayerId(0)).life, 21, "its replacement no longer doubles the gain");

    play(&mut game, [&[], &[]], |g| g.state.turn.number == 2);
    assert_eq!(game.chars(land).subtypes, SubtypeSet::NONE);
    assert!(game.chars(land).has(Keyword::Hexproof));
    assert!(!game.lost_all_abilities(land));
}

/// Each other land is an Island (Probe Sea); then Probe Swamp makes Probe Sea a Swamp. The effect is newer than Probe Sea's
/// ability, but takes it away (CR 305.7), so Probe Sea's effect depends on it and applies after it (CR 613.8a), and no longer
/// exists: the basic land is no Island. Making the basic land a Swamp keeps its basic supertype.
#[test]
fn a_resolved_effect_that_makes_a_land_a_swamp_takes_its_ability_to_set_other_lands_subtypes() {
    let mut game = deal(&["Probe Sea", "Probe Swamp", "Probe Swamp"], &["Probe Basic"]);
    play(&mut game, [&[("Probe Sea", "")], &[("Probe Basic", "")]], |g| settled(g, "Probe Basic"));
    let (sea, basic) = (on_battlefield(&game, "Probe Sea").unwrap(), on_battlefield(&game, "Probe Basic").unwrap());
    assert_eq!(game.chars(basic).subtypes, SubtypeSet::NONE.with(Subtype::Island), "the control");

    play(&mut game, [&[("Probe Swamp", "Probe Sea")], &[]], |g| g.state.stack.is_empty() && g.chars(sea).has_subtype(Subtype::Swamp));
    assert_eq!(game.chars(sea).subtypes, SubtypeSet::NONE.with(Subtype::Swamp));
    assert_eq!(game.chars(basic).subtypes, SubtypeSet::NONE, "Probe Sea's ability is gone");
    assert!(game.lost_all_abilities(sea));

    play(&mut game, [&[("Probe Swamp", "Probe Basic")], &[]], |g| g.state.stack.is_empty() && g.chars(basic).has_subtype(Subtype::Swamp));
    assert_eq!(game.chars(basic).subtypes, SubtypeSet::NONE.with(Subtype::Swamp));
    assert!(game.chars(basic).supertypes.basic, "its supertypes are unchanged");
}
