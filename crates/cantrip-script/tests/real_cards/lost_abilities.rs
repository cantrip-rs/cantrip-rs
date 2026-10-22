//! A permanent that has lost all its abilities (CR 613.1f) at the sites that ask whether it has: Ghalta the Immovable's "Creatures
//! you control can attack as though they didn't have defender" and its combat damage by toughness, Forge Anew's equip any time
//! you could cast an instant, and Ocelot Pride's ascend; and a Case's "Solved" static abilities - one of the layers and a play
//! rule, which no printed Case in the engine has both of - before and after the Case loses all its abilities.
//! No printed card in the engine takes the abilities of these permanents alone (Tishana's Tidebinder needs an ability of
//! theirs to counter, Humility makes every creature 1/1 and takes defender too), so a test card does here, beside the printed
//! cards and every other card the engine has.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "Target permanent loses all abilities."
fn silence(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::LoseAllAbilities, Duration::Indefinite);
    }
    Ok(())
}

fn permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id))
}

/// A zero-cost instant: target permanent loses all abilities.
static SILENCE: CardDef = CardDef { name: "Probe Silence", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(permanent)], silence), ..CardDef::BLANK };

/// A colorless artifact token, to count as a permanent.
static TRINKET: CardDef = CardDef { name: "Probe Trinket", has_cost: false, types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// "Create ten Probe Trinket tokens."
fn trinkets(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let trinket = g.card_id(&TRINKET);
    g.create_tokens_together(&[(trinket, item.controller); 10])?;
    Ok(())
}

/// A zero-cost sorcery: create ten Probe Trinkets.
static TRINKETS: CardDef = CardDef { name: "Probe Trinkets", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], trinkets), ..CardDef::BLANK };

fn always(_: &dyn Query, _: &Asker) -> bool {
    true
}

/// "Solved - Creatures you control get +1/+1" (CR 702.169b): a static ability of the layers, while its source is solved.
fn your_creatures_while_solved(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    cantrip_mechanics::solved::solved_in_layers(view, source)
        && view.state().obj(affected).controller == view.state().obj(source).controller
        && chars.is(CardType::Creature)
}

fn plus_one(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v + 1);
    chars.toughness = chars.toughness.map(|v| v + 1);
}

/// A zero-cost Case: "To solve - (always). Solved - Creatures you control get +1/+1. Solved - You may look at the top card of
/// your library any time."
static CASE: CardDef = CardDef {
    name: "Probe Case",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Case]),
    abilities: &[cantrip_mechanics::solved::to_solve(always, "To solve - Always.")],
    statics: &[static_ability(Layer::PtModify, your_creatures_while_solved, plus_one, "Solved - Creatures you control get +1/+1.")],
    play_rules: &[cantrip_mechanics::solved::play_rules(&[PlayRule::LookAtLibraryTop])],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SILENCE, &TRINKET, &TRINKETS, &PAUSE, &CASE])))
}

/// What Bot1 does, in order, each as soon as it is offered and its turn has come: cast a card (at a permanent named, for
/// Probe Silence), or attack Bot2 with the creatures named.
enum Step {
    Cast(u32, &'static str, Option<&'static str>),
    Attack(u32, &'static [&'static str]),
}

/// A game of Bot1's `hand` (then Islands) against Islands, played until every step is done and turn `until` has come.
/// Every question is shown to `watch` before it is answered.
fn play(hand: &[&str], steps: Vec<Step>, until: u32, mut watch: impl FnMut(&Game, &Request)) -> Game {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(hand);
    first.extend(names(&vec!["Island"; 30 - hand.len()]));
    let mut game = Game::new(table(), &Setup { decks: vec![first, names(&["Island"; 30])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut steps = steps.into_iter().peekable();
    let named = |game: &Game, name: &str| game.state.battlefield().iter().copied().find(|&id| game.name(id) == name);
    for _ in 0..2000 {
        game.run();
        if steps.peek().is_none() && game.state.turn.number >= until && game.state.stack.is_empty() {
            return game;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let turn = game.state.turn.number;
        watch(&game, &request);
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => match steps.peek() {
                Some(&Step::Cast(at, name, aimed)) if turn >= at => {
                    // A spell aimed at a permanent waits for the permanent to be there.
                    let target = aimed.map(|aimed| named(&game, aimed));
                    match options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)) {
                        Some(option) if target != Some(None) => {
                            steps.next();
                            let mut action = Action::of(option);
                            if let Some(Some(target)) = target {
                                action.targets = vec![Target::Object(target)];
                            }
                            Answer::Act(action)
                        }
                        _ => Answer::Pass,
                    }
                }
                _ => Answer::Pass,
            },
            RequestKind::PayCosts { options, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).text().starts_with("Mox")));
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
                Answer::Act(Action::of(confirm.or(mox).expect("a Mox to pay with, or the payment to confirm")))
            }
            RequestKind::DeclareAttackers { .. } if request.player == PlayerId(0) => {
                match steps.peek() {
                    Some(&Step::Attack(at, which)) if turn >= at => {
                        steps.next();
                        Answer::Attack(which.iter().map(|n| (named(&game, n).expect("the attacker"), Target::Player(PlayerId(1)))).collect())
                    }
                    _ => Answer::Attack(Vec::new()),
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

/// Bot1: Crystal Barricade (0/4, defender) on turn 1, Ghalta the Immovable ({4}{W}, for the Barricade's toughness) on turn 3,
/// the Mox Sapphire drawn then paying for it with the other four.
const GHALTA_HAND: [&str; 8] = ["Mox Pearl", "Mox Jet", "Mox Ruby", "Mox Emerald", "Crystal Barricade", "Ghalta the Immovable", "Probe Silence", "Mox Sapphire"];

fn ghalta_steps() -> Vec<Step> {
    vec![
        Step::Cast(1, "Mox Pearl", None),
        Step::Cast(1, "Mox Jet", None),
        Step::Cast(1, "Mox Ruby", None),
        Step::Cast(1, "Mox Emerald", None),
        Step::Cast(1, "Crystal Barricade", None),
        Step::Cast(3, "Mox Sapphire", None),
        Step::Cast(3, "Ghalta the Immovable", None),
    ]
}

/// Whether Bot1 is offered Crystal Barricade to attack with on turn 5, after Ghalta lost all its abilities or not.
fn barricade_may_attack(silenced: bool) -> bool {
    let mut steps = ghalta_steps();
    if silenced {
        steps.push(Step::Cast(5, "Probe Silence", Some("Ghalta the Immovable")));
    }
    let mut offered = None;
    play(&GHALTA_HAND, steps, 6, |game, request| {
        if let RequestKind::DeclareAttackers { eligible, .. } = &request.kind
            && request.player == PlayerId(0)
            && game.state.turn.number == 5
        {
            offered = Some(eligible.iter().any(|&id| game.name(id) == "Crystal Barricade"));
        }
    });
    offered.expect("Bot1 was asked to attack on turn 5")
}

/// A zero-cost instant that does nothing: something on the stack while Bot1 has priority.
static PAUSE: CardDef = CardDef { name: "Probe Pause", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], |_, _| Ok(())), ..CardDef::BLANK };

/// CR 602.5d, 613.1f: Forge Anew lets Bot1 activate equip abilities during its turn any time it could cast an instant - with a
/// spell of its own on the stack - and not once Forge Anew has lost all its abilities (`static_rules::instant_activation`).
#[test]
fn a_forge_anew_that_lost_its_abilities_lets_no_equip_at_instant_speed() {
    let equip_offered = |silenced: bool| {
        let mut steps = vec![
            Step::Cast(1, "Mox Pearl", None),
            Step::Cast(1, "Mox Jet", None),
            Step::Cast(1, "Mox Ruby", None),
            Step::Cast(1, "Forge Anew", None),
            Step::Cast(1, "Ornithopter", None),
            Step::Cast(1, "Paradise Mantle", None),
        ];
        if silenced {
            steps.push(Step::Cast(1, "Probe Silence", Some("Forge Anew")));
        }
        steps.push(Step::Cast(1, "Probe Pause", None));
        let mut offered = None;
        play(&["Mox Pearl", "Mox Jet", "Mox Ruby", "Forge Anew", "Ornithopter", "Paradise Mantle", "Probe Silence", "Probe Pause"][..], steps, 1, |game, request| {
            if let RequestKind::Priority { options } = &request.kind
                && request.player == PlayerId(0)
                && game.state.stack.iter().any(|i| i.kind == StackKind::Spell && game.name(i.object) == "Probe Pause")
            {
                offered = Some(options.iter().any(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Paradise Mantle")));
            }
        });
        offered.expect("Bot1 had priority with Probe Pause on the stack")
    };
    assert!(equip_offered(false), "Forge Anew lets no equip at instant speed");
    assert!(!equip_offered(true), "a Forge Anew that lost its abilities lets an equip at instant speed");
}

/// CR 702.3b, 613.1f: Ghalta's "can attack as though they didn't have defender" lets the Barricade attack, and not once Ghalta
/// has lost all its abilities (`static_rules::may_attack_despite_defender`).
#[test]
fn a_ghalta_that_lost_its_abilities_lets_no_defender_attack() {
    assert!(barricade_may_attack(false), "the Barricade may not attack beside Ghalta");
    assert!(!barricade_may_attack(true), "the Barricade may attack beside a Ghalta that lost its abilities");
}

/// CR 510.1a, 613.1f: the Barricade attacking beside Ghalta deals combat damage equal to its toughness, 4; when Ghalta has lost
/// all its abilities once the attack is declared, it deals its power, 0.
#[test]
fn a_ghalta_that_lost_its_abilities_after_the_attack_deals_no_damage_by_toughness() {
    let life = |silenced: bool| {
        let mut steps = ghalta_steps();
        steps.push(Step::Attack(5, &["Crystal Barricade"]));
        if silenced {
            steps.push(Step::Cast(5, "Probe Silence", Some("Ghalta the Immovable")));
        }
        play(&GHALTA_HAND, steps, 6, |_, _| {}).state.player(PlayerId(1)).life
    };
    assert_eq!(life(false), 16);
    assert_eq!(life(true), 20);
}

/// CR 702.131b, 613.1f: Ocelot Pride's ascend gives Bot1 the city's blessing once it controls ten permanents, and not when the
/// Pride has lost all its abilities first (`designations`).
#[test]
fn an_ocelot_pride_that_lost_its_abilities_gives_no_citys_blessing() {
    let blessed = |silenced: bool| {
        let mut steps = vec![Step::Cast(1, "Mox Pearl", None), Step::Cast(1, "Ocelot Pride", None)];
        if silenced {
            steps.push(Step::Cast(1, "Probe Silence", Some("Ocelot Pride")));
        }
        steps.push(Step::Cast(1, "Probe Trinkets", None));
        let game = play(&["Mox Pearl", "Ocelot Pride", "Probe Silence", "Probe Trinkets"], steps, 1, |_, _| {});
        assert!(game.state.battlefield().len() >= 12);
        game.state.player(PlayerId(0)).citys_blessing
    };
    assert!(blessed(false), "ten permanents and Ocelot Pride give no city's blessing");
    assert!(!blessed(true), "an Ocelot Pride that lost its abilities gives the city's blessing");
}

/// The Probe Case is solved at Bot1's first end step; on turn 3 it may first lose all its abilities. Whether it is solved, the
/// Ornithopter's power and toughness, and whether Bot1 may look at the top of its library.
fn case_on_turn_three(silenced: bool) -> (bool, (i32, i32), bool) {
    let mut steps = vec![Step::Cast(1, "Probe Case", None), Step::Cast(1, "Ornithopter", None)];
    if silenced {
        steps.push(Step::Cast(3, "Probe Silence", Some("Probe Case")));
    }
    let mut before = None;
    let game = play(&["Probe Case", "Ornithopter", "Probe Silence"], steps, 3, |game, _| {
        if let Some(thopter) = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Ornithopter")
            && game.state.turn.number == 1
            && before.is_none()
        {
            before = Some(game.chars(thopter).power);
        }
    });
    assert_eq!(before, Some(Some(0)), "the Ornithopter is a 0/2 before the Case is solved");
    let case = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Probe Case").expect("the Case");
    let thopter = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Ornithopter").expect("the Ornithopter");
    let c = game.chars(thopter);
    (game.state.obj(case).marks.solved, (i32::from(c.power.unwrap()), i32::from(c.toughness.unwrap())), game.may_look_at_library_top(PlayerId(0)))
}

/// CR 702.169b, 719.3c: once the Case is solved, its "Solved" static ability of the layers gives +1/+1 (`solved_in_layers`) and
/// its "Solved" play rule lets Bot1 look at the top of its library.
#[test]
fn a_solved_cases_static_abilities_apply() {
    assert_eq!(case_on_turn_three(false), (true, (1, 3), true));
}

/// CR 719.3b, 613.1f: a Case that loses all its abilities stays solved - a designation, not an ability - and its "Solved" static
/// abilities are gone with the rest: no +1/+1, and no look at the top of the library (`each_play_rule`).
#[test]
fn a_solved_case_that_lost_its_abilities_stays_solved_and_its_solved_abilities_are_gone() {
    assert_eq!(case_on_turn_three(true), (true, (0, 2), false));
}
