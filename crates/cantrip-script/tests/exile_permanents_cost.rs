//! "Exile N [permanents] you control" as a cost (CR 118.3, 601.2h, 602.2b), on a table of test-only cards: the permanents
//! are chosen as the cost is chosen, with a question that says they are exiled to pay and what they are; they are exiled
//! as it is paid; an activated ability's effect reads what it exiled as it last was (CR 608.2h), and an activated
//! ability's exiles are "exiled with" its source (CR 406.6, 607.2a) where a spell's additional cost's are not. With
//! nothing to exile the cost can't be paid and the ability is not offered (CR 118.3). The printed cases are The Soul
//! Stone's and Food Chain's "Exile a creature you control" and Soul Exchange's additional cost.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// "You gain life equal to the exiled creature's power."
fn gain_exiled_power(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.cost_objects(item).first().copied();
    if let Some(exiled) = exiled {
        let power = g.chars(exiled).power.unwrap_or(0) as i32;
        g.gain_life(item.controller, i64::from(power));
    }
    Ok(())
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// An artifact with "Exile a creature you control: You gain life equal to the exiled creature's power."
static STONE: CardDef = CardDef {
    name: "Probe Stone",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(exile_permanents(1, creature), gain_exiled_power, "Exile a creature you control: You gain life equal to the exiled creature's power.")],
    ..CardDef::BLANK
};

/// A sorcery with "As an additional cost to cast this spell, exile a creature you control. You gain 1 life."
static EXCHANGE: CardDef = CardDef {
    name: "Probe Exchange",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], gain_one),
    additional_cost: Some(Cost::exile_permanents(1, creature)),
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef { name: "Probe Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static GIANT: CardDef = CardDef { name: "Probe Giant", types: TypeSet::of(&[CardType::Creature]), power: Some(5), toughness: Some(5), ..CardDef::BLANK };
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&STONE, &EXCHANGE, &BEAR, &GIANT, &FILLER]))
}

/// What the cost asked: its purpose, the kind of permanent it named, how many and among which.
struct Asked {
    purpose: Purpose,
    kind: &'static str,
    count: u32,
    among: Vec<String>,
}

/// Plays Bot1's turn 1: each (kind, name, index) in order, as it is offered, and the permanents a cost chooses by the name
/// `exile` gives; everything else passes. The game, and the cost's questions.
fn play(first: &[&str], steps: &[(ActionKind, &str, u8)], exile: &str) -> (Game, Vec<Asked>) {
    let filler = table().find("Filler").unwrap();
    let mut deck: Vec<CardId> = first.iter().map(|n| table().find(n).unwrap()).collect();
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let others = vec![filler; 30];
    let mut game = Game::new(table(), &Setup { decks: vec![deck, others], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut asked = Vec::new();
    let mut todo = steps.iter().peekable();
    for _ in 0..200 {
        game.run();
        if todo.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            break;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let option = todo.peek().and_then(|&&(kind, name, index)| {
                    options.iter().find(|o| o.kind == kind && o.index == index && o.source.is_some_and(|s| game.name(s) == name))
                });
                match option {
                    Some(option) => {
                        todo.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCards { purpose, rule, .. } => {
                let eligible = first_choices(&game, &request);
                let count = rule.count.most().unwrap_or(0);
                asked.push(Asked { purpose: *purpose, kind: rule.kind, count, among: eligible.iter().map(|&c| game.name(c).to_string()).collect() });
                Answer::Cards(eligible.iter().copied().filter(|&c| game.name(c) == exile).collect())
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(todo.peek().is_none(), "every step was offered");
    (game, asked)
}

fn on_battlefield(game: &Game, name: &str) -> Option<ObjectId> {
    game.state.battlefield().iter().copied().find(|&id| game.name(id) == name)
}

fn in_exile(game: &Game, name: &str) -> Option<ObjectId> {
    game.state.zone(Zone::exile(PlayerId(0))).iter().copied().find(|&id| game.name(id) == name)
}

/// The creature chosen of two is exiled, the other stays; the effect reads the exiled one's power (5, not the Bear's 2);
/// the question was an exile to pay among the creatures, naming them as creatures; the exiled card is linked to the stone.
#[test]
fn an_activated_ability_exiles_the_creature_chosen_and_reads_it() {
    let (game, asked) = play(
        &["Probe Stone", "Probe Bear", "Probe Giant"],
        &[
            // The stone last: a creature still on the stack as the ability is activated could not be exiled.
            (ActionKind::CastSpell, "Probe Bear", 0),
            (ActionKind::CastSpell, "Probe Giant", 0),
            (ActionKind::CastSpell, "Probe Stone", 0),
            (ActionKind::ActivateAbility, "Probe Stone", 0),
        ],
        "Probe Giant",
    );
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].purpose, Purpose::ExileToPay);
    assert_eq!(asked[0].kind, "creature");
    assert_eq!(asked[0].count, 1);
    assert_eq!(asked[0].among, vec!["Probe Bear".to_string(), "Probe Giant".to_string()]);
    assert!(on_battlefield(&game, "Probe Giant").is_none());
    assert!(on_battlefield(&game, "Probe Bear").is_some());
    let giant = in_exile(&game, "Probe Giant").expect("the Giant in exile");
    let stone = on_battlefield(&game, "Probe Stone").expect("the stone");
    assert_eq!(game.state.obj(giant).exiled.map(|l| l.by), Some(stone));
    assert_eq!(game.state.player(PlayerId(0)).life, 25);
}

/// CR 118.3: with no creature to exile the ability is not offered (the stone is the only permanent besides the land).
#[test]
fn the_ability_is_not_offered_without_a_creature_to_exile() {
    let (mut game, _) = play(&["Probe Stone"], &[(ActionKind::CastSpell, "Probe Stone", 0)], "");
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    assert!(!options.iter().any(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Probe Stone")));
}

/// A spell's additional cost exiles the creature as the spell is cast (CR 601.2h): not linked to the spell, which is no
/// ability (CR 607.2a), and the spell resolves.
#[test]
fn a_spells_additional_cost_exiles_a_creature_you_control() {
    let (game, asked) = play(
        &["Probe Bear", "Probe Exchange"],
        &[(ActionKind::CastSpell, "Probe Bear", 0), (ActionKind::CastSpell, "Probe Exchange", 0)],
        "Probe Bear",
    );
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].purpose, Purpose::ExileToPay);
    let bear = in_exile(&game, "Probe Bear").expect("the Bear in exile");
    assert!(game.state.obj(bear).exiled.is_none());
    assert_eq!(game.state.player(PlayerId(0)).life, 21);
}

/// CR 601.2h, 118.3: the spell can't be cast with no creature to exile.
#[test]
fn the_spell_is_not_castable_without_a_creature_to_exile() {
    let (mut game, _) = play(&["Probe Stone", "Probe Exchange"], &[(ActionKind::CastSpell, "Probe Stone", 0)], "");
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    assert!(!options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Probe Exchange")));
}
