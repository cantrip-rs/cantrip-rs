//! The Fantasticar's "Whenever you cast your fourth noncreature spell each turn, you may sacrifice The Fantasticar. If you do,
//! create four 4/4 ... Construct ... tokens": while it can't be sacrificed (CR 101.2, 701.21a: a resolved effect says so) the
//! option can't be taken, so it is not offered and no Construct is made; without the effect it is offered and taken (the
//! positive control). No printed card in the engine keeps an artifact that isn't Hithlain Rope from being sacrificed, so a
//! test instant does.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id))
}

/// "Target permanent can't be sacrificed this turn."
fn guard(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::CantBeSacrificed, Duration::EndOfTurn);
    }
    Ok(())
}

/// A zero-cost instant: target permanent can't be sacrificed this turn.
static GUARD: CardDef = CardDef { name: "Probe Guard", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(permanent)], guard), ..CardDef::BLANK };

/// A zero-cost instant that does nothing: a noncreature spell to count.
static PAUSE: CardDef = CardDef { name: "Probe Pause", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], |_, _| Ok(())), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&GUARD, &PAUSE])))
}

/// Bot1 casts three Moxen and The Fantasticar on turn 1, then on turn 3 four noncreature spells - Probe Guard at The
/// Fantasticar first when `guarded`, else a fourth Probe Pause. It never animates the Fantasticar, and sacrifices it when
/// asked. The game, and whether the sacrifice was asked.
fn play(guarded: bool) -> (Game, bool) {
    let find = |n: &str| table().find(n).unwrap_or_else(|| panic!("no card {n}"));
    let first = if guarded { "Probe Guard" } else { "Probe Pause" };
    let order = ["Mox Ruby", "Mox Jet", "Mox Emerald", "The Fantasticar", first, "Probe Pause", "Probe Pause", "Probe Pause"];
    let mut deck: Vec<CardId> = order.iter().map(|n| find(n)).collect();
    deck.extend(std::iter::repeat(find("Island")).take(30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![find("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = order.iter().enumerate().peekable();
    let mut asked = false;
    for _ in 0..2000 {
        game.run();
        if casts.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            return (game, asked);
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.stack.is_empty() => {
                // The four spells wait for turn 3.
                let next = casts.peek().filter(|&&(i, _)| i < 4 || game.state.turn.number >= 3);
                match next.and_then(|&(_, &n)| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == n))) {
                    Some(o) => {
                        let mut action = Action::of(o);
                        if game.name(o.source.unwrap()) == "Probe Guard" {
                            let car = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "The Fantasticar").expect("The Fantasticar");
                            action.targets = vec![Target::Object(car)];
                        }
                        casts.next();
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).starts_with("Mox")));
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::May { purpose: Purpose::Ask(q), .. } if q.verb == "SacrificeForTokens" => {
                asked = true;
                Answer::Yes
            }
            RequestKind::May { .. } => Answer::No,
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

fn constructs(game: &Game) -> usize {
    game.state.battlefield().iter().filter(|&&id| game.name(id).starts_with("Construct")).count()
}

#[test]
fn the_fantasticar_that_cant_be_sacrificed_is_not_offered_and_makes_no_constructs() {
    let (game, asked) = play(true);
    assert!(!asked);
    assert_eq!(constructs(&game), 0);
    assert!(game.state.battlefield().iter().any(|&id| game.name(id) == "The Fantasticar"));
    let (game, asked) = play(false);
    assert!(asked);
    assert_eq!(constructs(&game), 4);
}
