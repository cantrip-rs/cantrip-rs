//! "Can't be sacrificed" (CR 101.2, 701.21a, 118.3), on a table of test-only cards, in each of its three forms: a permanent's
//! own ability (`Keyword::CantBeSacrificed`: Hithlain Rope, Alexios), a resolved effect (`EffectKind::CantBeSacrificed`:
//! Slicer, Hired Muscle's "this turn"), and another permanent's static ability (`CardDef::sacrifice_bans`: Assault Suit,
//! Garland). Such a permanent pays no sacrifice cost, an instruction to sacrifice it does nothing, and a choice of what to
//! sacrifice can't take it (the rulings: "You can't try to sacrifice [it], fail, and therefore ignore the effect"). The
//! ability is lost with "loses all abilities"; the effect is not; a static ability ends with its source's abilities. The
//! printed case is Hithlain Rope (`scenarios/hithlain-rope-*.cantrip`).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn other_artifact(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && q.chars(id).is(CardType::Artifact)
}

fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn other_artifacts(g: &dyn Rules, item: &StackItem) -> Vec<ObjectId> {
    let asker = Asker { instance: None, source: item.object, controller: item.controller, by: None, spell: false };
    g.permanents_of(item.controller).into_iter().filter(|&id| other_artifact(g, &asker, id)).collect()
}

/// "Sacrifice an artifact other than this one" as an instruction, taken without a question when there is no choice.
fn sacrifice_an_artifact(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let eligible = other_artifacts(g, item);
    cantrip_mechanics::sacrifice_one_of(g, item.controller, item.object, 1, eligible).map(drop)
}

/// "Sacrifice two other artifacts", always asked: what the question offers is the test.
fn sacrifice_two_asked(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let eligible = other_artifacts(g, item);
    let chosen = cantrip_mechanics::choose::cards(g, item.controller, item.object, Purpose::Sacrifice, eligible, 2, false)?;
    g.sacrifice(item.controller, &chosen);
    Ok(())
}

/// "Each other artifact you control loses all abilities until end of turn."
fn hush(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    for id in other_artifacts(g, item) {
        g.add_effect(item.object, id, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    }
    Ok(())
}

/// "Each other artifact you control can't be sacrificed this turn."
fn guard(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    for id in other_artifacts(g, item) {
        g.add_effect(item.object, id, EffectKind::CantBeSacrificed, Duration::EndOfTurn);
    }
    Ok(())
}

/// "Sacrifice a creature" as an instruction.
fn sacrifice_a_creature(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::sacrifice_chosen(g, item.controller, item.object, 1, |c| c.is(CardType::Creature)).map(drop)
}

/// "Sacrifice another artifact", asked as a stage of its own (`Game::ask_then`), as a card written in explicit stages asks.
fn sacrifice_an_artifact_staged(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    match g.stage() {
        0 => {
            let eligible = other_artifacts(g, item);
            let ask = RequestKind::choose_cards(item.object, Purpose::Sacrifice, eligible.clone(), eligible, 1, false, "");
            Err(g.ask_then(1, item.controller, ask))
        }
        _ => {
            if let Some(Answer::Cards(chosen)) = g.staged_answer() {
                g.sacrifice(item.controller, &chosen);
            }
            Ok(())
        }
    }
}

static LAB: CardDef = CardDef {
    name: "Probe Lab",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(Cost::sacrifice(1, other_artifact), nothing, "Sacrifice another artifact: Nothing happens."),
        ability(Cost::NONE, sacrifice_an_artifact, "Sacrifice another artifact."),
        ability(Cost::NONE, sacrifice_two_asked, "Sacrifice two other artifacts."),
        ability(Cost::NONE, hush, "Each other artifact you control loses all abilities until end of turn."),
        ability(Cost::NONE, guard, "Each other artifact you control can't be sacrificed this turn."),
        ability(Cost::sacrifice(1, creature), nothing, "Sacrifice a creature: Nothing happens."),
        ability(Cost::NONE, sacrifice_a_creature, "Sacrifice a creature."),
        ability(Cost::NONE, sacrifice_an_artifact_staged, "Sacrifice another artifact, asked in a stage of its own."),
    ],
    ..CardDef::BLANK
};

/// "This artifact can't be sacrificed. Sacrifice this artifact: Nothing happens."
static ROPE: CardDef = CardDef {
    name: "Probe Rope",
    types: TypeSet::of(&[CardType::Artifact]),
    keywords: Keywords::of(&[Keyword::CantBeSacrificed]),
    abilities: &[ability(SACRIFICE_THIS, nothing, "Sacrifice this artifact: Nothing happens.")],
    ..CardDef::BLANK
};

/// "Sacrifice this artifact: Nothing happens."
static PLAIN: CardDef = CardDef {
    name: "Probe Plain",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(SACRIFICE_THIS, nothing, "Sacrifice this artifact: Nothing happens.")],
    ..CardDef::BLANK
};

fn creatures_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && q.chars(id).is(CardType::Creature)
}

/// "Creatures you control can't be sacrificed."
static SUIT: CardDef = CardDef {
    name: "Probe Suit",
    types: TypeSet::of(&[CardType::Artifact]),
    sacrifice_bans: &[creatures_you_control],
    ..CardDef::BLANK
};

fn enchantments_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && q.chars(id).is(CardType::Enchantment)
}

/// "Enchantments you control can't be sacrificed."
static SHRINE: CardDef = CardDef {
    name: "Probe Shrine",
    types: TypeSet::of(&[CardType::Artifact]),
    sacrifice_bans: &[enchantments_you_control],
    ..CardDef::BLANK
};

/// A Saga of one chapter: "I - Nothing happens."
static SAGA: CardDef = CardDef {
    name: "Probe Saga",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Saga]),
    abilities: &[chapter(1, nothing, "I - Nothing happens.")],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&LAB, &ROPE, &PLAIN, &SUIT, &SHRINE, &SAGA, &BEAR, &FILLER]))
}

/// What a played game asked of card choices: each question's choosable cards, by name, and its count.
type Asked = Vec<(Vec<String>, Count)>;

/// Bot1 casts each of `cast` in order on turn 1, then activates each of Probe Lab's abilities in `activate`, in order; every
/// card choice takes the first choosable cards. The game, and the card choices asked.
fn play(cast: &[&str], activate: &[u8]) -> (Game, Asked) {
    let filler = table().find("Filler").unwrap();
    let mut deck: Vec<CardId> = cast.iter().map(|n| table().find(n).unwrap()).collect();
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = cast.iter().peekable();
    let mut activations = activate.iter().peekable();
    let mut asked = Asked::new();
    for _ in 0..200 {
        game.run();
        let asking = game.pending().is_some_and(|r| !matches!(r.kind, RequestKind::Priority { .. }));
        if casts.peek().is_none() && activations.peek().is_none() && game.state.stack.is_empty() && !asking {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let named = |o: &&ActionOption, name: &str| o.source.is_some_and(|s| game.name(s) == name);
                // Each step as soon as it is offered once the one before it has resolved.
                let option = if !game.state.stack.is_empty() {
                    None
                } else if let Some(&&name) = casts.peek() {
                    options.iter().find(|o| o.kind == ActionKind::CastSpell && named(o, name)).inspect(|_| {
                        casts.next();
                    })
                } else if let Some(&&index) = activations.peek() {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.index == index && named(o, "Probe Lab")).inspect(|_| {
                        activations.next();
                    })
                } else {
                    None
                };
                option.map_or(Answer::Pass, |o| Answer::Act(Action::of(o)))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCards { cards, rule, .. } => {
                let choosable: Vec<ObjectId> = rule.among.clone().unwrap_or_else(|| cards.clone());
                asked.push((choosable.iter().map(|&id| game.name(id).to_string()).collect(), rule.count));
                Answer::Cards(choosable.into_iter().take(rule.count.most().unwrap_or(0) as usize).collect())
            }
            RequestKind::PayCosts { options, .. } => match options.iter().find(|o| o.kind == ActionKind::AssignMana) {
                Some(pay) => Answer::Act(Action::of(pay)),
                None => panic!("nothing pays the cost: {options:?}"),
            },
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(casts.peek().is_none() && activations.peek().is_none(), "every step was offered");
    (game, asked)
}

/// The abilities of the first permanent named `name` Bot1 is offered at its next priority.
fn offered(game: &mut Game, name: &str) -> Vec<u8> {
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    options.iter().filter(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == name)).map(|o| o.index).collect()
}

/// The names of the cards in Bot1's zone.
fn names(game: &Game, zone: Zone) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = game.state.zone(zone).iter().map(|&id| game.name(id)).filter(|&n| n != "Filler").collect();
    names.sort();
    names
}

fn battlefield(game: &Game) -> Vec<&'static str> {
    names(game, Zone::BATTLEFIELD)
}

fn graveyard(game: &Game) -> Vec<&'static str> {
    names(game, Zone::graveyard(PlayerId(0)))
}

/// CR 118.3, 701.21a: a permanent that can't be sacrificed pays no cost that sacrifices it - neither its own "sacrifice
/// this" nor another's "sacrifice an artifact" - while one that can pays both (the positive control).
#[test]
fn it_pays_no_sacrifice_cost() {
    let (mut game, _) = play(&["Probe Lab", "Probe Rope"], &[]);
    assert_eq!(offered(&mut game, "Probe Rope"), Vec::<u8>::new());
    assert!(!offered(&mut game, "Probe Lab").contains(&0));
    let (mut game, _) = play(&["Probe Lab", "Probe Plain"], &[]);
    assert_eq!(offered(&mut game, "Probe Plain"), vec![0]);
    assert!(offered(&mut game, "Probe Lab").contains(&0));
}

/// CR 101.2: "sacrifice an artifact" with only one that can't be sacrificed sacrifices nothing; with another there, that one
/// is sacrificed, and nobody is asked, as there is no choice.
#[test]
fn an_instruction_skips_it_and_takes_another() {
    let (game, asked) = play(&["Probe Lab", "Probe Rope"], &[1]);
    assert_eq!(battlefield(&game), vec!["Probe Lab", "Probe Rope"]);
    assert!(asked.is_empty());
    let (game, asked) = play(&["Probe Lab", "Probe Rope", "Probe Plain"], &[1]);
    assert_eq!(battlefield(&game), vec!["Probe Lab", "Probe Rope"]);
    assert_eq!(graveyard(&game), vec!["Probe Plain"]);
    assert!(asked.is_empty());
}

/// A choice of what to sacrifice offers only what can be sacrificed; with fewer of those than it takes, it takes as many as
/// it can (CR 701.21a), and with none it is not asked.
#[test]
fn a_choice_leaves_it_out() {
    let (game, asked) = play(&["Probe Lab", "Probe Rope", "Probe Plain", "Probe Plain"], &[2]);
    assert_eq!(asked, vec![(vec!["Probe Plain".to_string(), "Probe Plain".to_string()], Count::Exactly(2))]);
    assert_eq!(graveyard(&game), vec!["Probe Plain", "Probe Plain"]);
    let (game, asked) = play(&["Probe Lab", "Probe Rope", "Probe Rope", "Probe Plain"], &[2]);
    assert_eq!(asked, vec![(vec!["Probe Plain".to_string()], Count::AsManyAsCan(2))]);
    assert_eq!(graveyard(&game), vec!["Probe Plain"]);
    let (game, asked) = play(&["Probe Lab", "Probe Rope", "Probe Rope", "Probe Rope"], &[2]);
    assert!(asked.is_empty());
    assert_eq!(graveyard(&game), Vec::<&str>::new());
}

/// CR 613.1f: "this artifact can't be sacrificed" is an ability, which "loses all abilities" takes: the Rope is then
/// sacrificed like any artifact.
#[test]
fn the_ability_is_lost_with_its_abilities() {
    let (game, _) = play(&["Probe Lab", "Probe Rope"], &[3, 1]);
    assert_eq!(graveyard(&game), vec!["Probe Rope"]);
}

/// "It can't be sacrificed this turn" is an effect, not an ability (Slicer's): an artifact under it that loses all its
/// abilities still can't be sacrificed.
#[test]
fn the_effect_outlasts_its_abilities() {
    let (game, _) = play(&["Probe Lab", "Probe Plain"], &[4, 3, 1]);
    assert_eq!(battlefield(&game), vec!["Probe Lab", "Probe Plain"]);
    let (game, _) = play(&["Probe Lab", "Probe Plain"], &[3, 1]);
    assert_eq!(graveyard(&game), vec!["Probe Plain"]);
}

/// Another permanent's "creatures you control can't be sacrificed" (Assault Suit's, Garland's): the Bear pays no cost and
/// an instruction leaves it; once the Suit loses its abilities, its rule is gone (CR 613.1f) and the Bear is sacrificed.
#[test]
fn a_static_ability_bans_it_while_its_source_has_its_abilities() {
    let (mut game, _) = play(&["Probe Lab", "Probe Suit", "Probe Bear"], &[6]);
    assert_eq!(battlefield(&game), vec!["Probe Bear", "Probe Lab", "Probe Suit"]);
    assert!(!offered(&mut game, "Probe Lab").contains(&5));
    let (game, _) = play(&["Probe Lab", "Probe Suit", "Probe Bear"], &[3, 6]);
    assert_eq!(graveyard(&game), vec!["Probe Bear"]);
}

/// CR 714.4, 704.5s: "that Saga's controller sacrifices it" is a sacrifice, so a Saga that can't be sacrificed stays on the
/// battlefield with its last chapter done; without the ban it goes (the positive control).
#[test]
fn a_saga_that_cant_be_sacrificed_stays() {
    let (game, _) = play(&["Probe Shrine", "Probe Saga"], &[]);
    assert_eq!(battlefield(&game), vec!["Probe Saga", "Probe Shrine"]);
    let (game, _) = play(&["Probe Saga"], &[]);
    assert_eq!(graveyard(&game), vec!["Probe Saga"]);
}

/// A question asked as a stage of its own (`Game::ask_then`) is held to the same rule: beside a Plain the choice offers only
/// the Plain, and with only the Rope nothing is asked and the next stage gets an empty answer - nothing is sacrificed.
#[test]
fn a_staged_choice_leaves_it_out() {
    let (game, asked) = play(&["Probe Lab", "Probe Rope", "Probe Plain"], &[7]);
    assert_eq!(asked, vec![(vec!["Probe Plain".to_string()], Count::Exactly(1))]);
    assert_eq!(graveyard(&game), vec!["Probe Plain"]);
    let (game, asked) = play(&["Probe Lab", "Probe Rope"], &[7]);
    assert!(asked.is_empty());
    assert_eq!(battlefield(&game), vec!["Probe Lab", "Probe Rope"]);
}
