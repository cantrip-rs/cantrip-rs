//! A face-down permanent turned face up by an effect (CR 708.8, 701.58g): `Game::turn_face_up` with the effect's controller as
//! the player `Event::TurnedFaceUp` records, and a cloaked instant that would turn face up is revealed and stays face down
//! (CR 701.58g). No printed card in the engine turns a permanent face up as an effect, so a test card does here, beside the
//! printed Cryptic Coat and every other card the engine has.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "Turn target face-down permanent face up."
fn unveil(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.turn_face_up(id, item.controller);
    }
    Ok(())
}

fn face_down(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.obj(id).face_down)
}

static UNVEIL: CardDef = CardDef { name: "Probe Unveil", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(face_down)], unveil), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&UNVEIL])))
}

/// Bot1 casts three Moxen and Cryptic Coat, which cloaks `cloaked` (the top card once the hand is drawn), then Probe Unveil
/// at the cloaked permanent. The game once the Unveil has resolved, and the permanent.
fn unveiled(cloaked: &str) -> (Game, ObjectId) {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(&["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Cryptic Coat", "Probe Unveil", "Island", "Island", cloaked]);
    first.extend(names(&["Island"; 12]));
    let mut game = Game::new(table(), &Setup { decks: vec![first, names(&["Island"; 20])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = ["Mox Sapphire", "Mox Sapphire", "Mox Sapphire", "Cryptic Coat"].into_iter().peekable();
    let hidden = |game: &Game| game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).face_down);
    let mut cast = false;
    for _ in 0..200 {
        game.run();
        if cast && game.state.stack.is_empty() {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let next = casts.peek().and_then(|&name| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)));
                if let Some(option) = next {
                    casts.next();
                    Answer::Act(Action::of(option))
                } else if let (Some(permanent), false, true) = (hidden(&game), cast, game.state.stack.is_empty()) {
                    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == "Probe Unveil")).expect("the Unveil");
                    cast = true;
                    let mut action = Action::of(option);
                    action.targets = vec![Target::Object(permanent)];
                    Answer::Act(action)
                } else {
                    Answer::Pass
                }
            }
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Mox Sapphire"));
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::Priority { .. } => Answer::Pass,
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(cast, "Probe Unveil was never cast");
    let permanent = *game.state.battlefield().iter().find(|&&id| game.state.obj(id).card == table().find(cloaked).unwrap()).expect("the cloaked card");
    (game, permanent)
}

/// A cloaked creature card is turned face up by the effect, its controller the player recorded as turning it up.
#[test]
fn an_effect_turns_a_cloaked_creature_face_up_as_its_controller() {
    let (game, permanent) = unveiled("Llanowar Elves");
    assert!(!game.state.obj(permanent).face_down);
    assert_eq!(game.name(permanent), "Llanowar Elves");
    let turned: Vec<PlayerId> = game.log.iter().filter_map(|r| match r.event {
        Event::TurnedFaceUp { permanent: p, player } if p == permanent => Some(player),
        _ => None,
    }).collect();
    assert_eq!(turned, [PlayerId(0)]);
}

/// A cloaked instant card that would turn face up is revealed and stays face down (CR 701.58g).
#[test]
fn an_effect_reveals_a_cloaked_instant_and_leaves_it_face_down() {
    let (game, permanent) = unveiled("Lightning Bolt");
    assert!(game.state.obj(permanent).face_down);
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::TurnedFaceUp { permanent: p, .. } if p == permanent)));
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Revealed { card, .. } if card == permanent)), "the instant was not revealed");
}
