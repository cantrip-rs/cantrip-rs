//! Floodpits Drowner targeting itself: "{1}{U}, {T}: Shuffle Floodpits Drowner and target creature with a stun counter on it
//! into their owners' libraries." Nothing keeps the Drowner from being its own target, and then it is one object shuffled
//! in once (CR 701.24a). No printed card in the engine puts a stun counter on an untapped permanent - each one taps it too -
//! so a test card does here: the printed Drowner, with every card the engine has, beside "Probe Stunner".

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "Put a stun counter on target creature."
fn stun(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_counters(id, CounterKind::Stun, 1);
    }
    Ok(())
}

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// A zero-cost instant: put a stun counter on target creature, leaving it untapped.
static STUNNER: CardDef = CardDef { name: "Probe Stunner", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature)], stun), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&STUNNER])))
}

#[test]
fn floodpits_drowner_targeting_itself_is_shuffled_in_once() {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(&["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Floodpits Drowner", "Probe Stunner", "Island"]);
    first.extend(names(&["Island"; 13]));
    let second = names(&["Island"; 20]);
    let mut game = Game::new(table(), &Setup { decks: vec![first, second], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    // Turn 1: four Moxen and the Drowner; turn 3: the Stunner on the Drowner, then its ability at itself.
    let mut casts = ["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Floodpits Drowner"].into_iter().peekable();
    let drowner = |game: &Game| game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Floodpits Drowner");
    let mut activated = false;
    for _ in 0..400 {
        game.run();
        if activated && game.state.stack.is_empty() {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let cast = casts.peek().and_then(|&name| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)));
                let stunned = drowner(&game).is_some_and(|d| game.state.obj(d).counters(CounterKind::Stun) > 0);
                if let Some(option) = cast {
                    casts.next();
                    Answer::Act(Action::of(option))
                } else if game.state.turn.number == 3 && game.state.stack.is_empty() && !stunned && !activated {
                    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Probe Stunner")).expect("the Stunner");
                    let mut action = Action::of(option);
                    action.targets = vec![Target::Object(drowner(&game).unwrap())];
                    Answer::Act(action)
                } else if stunned && !activated && game.state.stack.is_empty() {
                    match options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Floodpits Drowner")) {
                        Some(option) => {
                            activated = true;
                            let mut action = Action::of(option);
                            if option.targets_required > 0 {
                                action.targets = vec![Target::Object(drowner(&game).unwrap())];
                            }
                            Answer::Act(action)
                        }
                        None => panic!("the Drowner's ability is not offered: {options:?}"),
                    }
                } else {
                    Answer::Pass
                }
            }
            RequestKind::ChooseTarget { candidates, .. } => {
                let d = drowner(&game).expect("the Drowner");
                assert!(candidates.contains(&Target::Object(d)), "the Drowner is no candidate for its own target");
                Answer::Targets(vec![Target::Object(d)])
            }
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Mox Sapphire"));
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(activated, "the Drowner's ability was never offered");
    assert!(drowner(&game).is_none(), "the Drowner is still on the battlefield");
    let library = game.state.zone(Zone::library(PlayerId(0)));
    let drowners = library.iter().filter(|&&id| game.name(id) == "Floodpits Drowner").count();
    assert_eq!(drowners, 1, "the Drowner is in its owner's library once");
}
