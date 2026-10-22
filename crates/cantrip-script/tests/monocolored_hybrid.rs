//! Monocolored hybrid symbols (CR 107.4e), {2/W}: each paid with one mana of its color or two generic mana, the choice
//! announced as the spell is cast (CR 601.2b, 118.13a), on a table of test-only cards. "Probe Severance" costs
//! {2/W}{2/B}{2/G}, as Kin-Tree Severance does; it is announced as its nonhybrid equivalent, which the mana then pays.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// {2/W}{2/B}{2/G}: "You gain 1 life."
static SEVERANCE: CardDef = CardDef {
    name: "Probe Severance",
    cost: mana("{2/W}{2/B}{2/G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], gain_one),
    ..CardDef::BLANK
};

static WHITE: CardDef = CardDef {
    name: "Probe White Mox",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::W), "{T}: Add {W}.")],
    ..CardDef::BLANK
};

static BLACK: CardDef = CardDef {
    name: "Probe Black Mox",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::B), "{T}: Add {B}.")],
    ..CardDef::BLANK
};

static GREEN: CardDef = CardDef {
    name: "Probe Green Mox",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

static BLUE: CardDef = CardDef {
    name: "Probe Blue Mox",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&SEVERANCE, &WHITE, &BLACK, &GREEN, &BLUE, &FILLER]))
}

/// What a game played by `play` saw: the announcement's advice and answer, and the game.
struct Played {
    game: Game,
    /// The `affordable` advice of each announcement asked.
    advised: Vec<Vec<u8>>,
    /// Whether the Severance was offered at the last priority Bot1 had.
    offered: bool,
}

/// Bot1's opening hand is `hand`; it casts every Mox, taps each for mana, then casts the Severance if offered, answering
/// the announcement with `answer` (given the advice). Everything else passes.
fn play(hand: &[&str], answer: fn(&[u8]) -> Vec<u8>) -> Played {
    let filler = table().find("Filler").unwrap();
    let mut deck: Vec<CardId> = hand.iter().map(|n| table().find(n).unwrap()).collect();
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut advised = Vec::new();
    let mut offered = false;
    let mut cast = false;
    let mut acted = false;
    for _ in 0..200 {
        game.run();
        // Once the Severance is cast and has resolved, and no question about it is waiting.
        if cast && game.state.stack.is_empty() && matches!(game.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. })) {
            break;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let reply = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let named = |kind: ActionKind, prefix: &str| {
                    options.iter().find(|o| o.kind == kind && o.index == 0 && o.source.is_some_and(|s| game.name(s).text().starts_with(prefix)))
                };
                offered = named(ActionKind::CastSpell, "Probe Severance").is_some();
                match named(ActionKind::CastSpell, "Probe ").filter(|o| game.name(o.source.unwrap()).text().ends_with("Mox"))
                    .or_else(|| named(ActionKind::ActivateAbility, "Probe "))
                    .or_else(|| named(ActionKind::CastSpell, "Probe Severance"))
                {
                    Some(option) => {
                        acted = true;
                        cast |= game.name(option.source.unwrap()) == "Probe Severance";
                        Answer::Act(Action::of(option))
                    }
                    // Nothing to do yet before the main phase; nothing left to do after acting.
                    None if acted && game.state.stack.is_empty() => break,
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseModes { affordable, .. } => {
                advised.push(affordable.clone());
                Answer::Modes(answer(affordable))
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, reply).unwrap();
    }
    Played { game, advised, offered }
}

fn spent_all(game: &Game) -> bool {
    game.state.player(PlayerId(0)).pool.is_empty()
}

/// CR 107.4e, 601.2b: with a white, a black and a green Mox, each {2/W} is announced as its color and the three Moxen pay
/// {W}{B}{G}; the advice is to pay none with {2}.
#[test]
fn each_symbol_paid_with_its_color() {
    let played = play(&["Probe Severance", "Probe White Mox", "Probe Black Mox", "Probe Green Mox"], |advice| advice.to_vec());
    assert_eq!(played.advised, vec![Vec::<u8>::new()]);
    assert_eq!(played.game.state.player(PlayerId(0)).life, 21);
    assert!(spent_all(&played.game));
}

/// CR 107.4e: with a white Mox and four blue ones, the {2/B} and the {2/G} are announced as {2} each, which the blue mana
/// pays, and the {2/W} as {W}: five mana for a spell of mana value 6. The advice says which.
#[test]
fn symbols_without_their_color_paid_with_two_generic() {
    let hand = ["Probe Severance", "Probe White Mox", "Probe Blue Mox", "Probe Blue Mox", "Probe Blue Mox", "Probe Blue Mox"];
    let played = play(&hand, |advice| advice.to_vec());
    assert_eq!(played.advised, vec![vec![1, 2]]);
    assert_eq!(played.game.state.player(PlayerId(0)).life, 21);
    assert!(spent_all(&played.game));
}

/// The refusal: with a white Mox and three blue ones (one short of {W}{2}{2}), the Severance is not offered; the
/// positive control is the case above, with a fourth.
#[test]
fn not_offered_one_mana_short() {
    let played = play(&["Probe Severance", "Probe White Mox", "Probe Blue Mox", "Probe Blue Mox", "Probe Blue Mox"], |advice| advice.to_vec());
    assert!(!played.offered);
    assert!(played.advised.is_empty());
    assert_eq!(played.game.state.player(PlayerId(0)).life, 20);
}

/// The mana value counts each {2/W} as 2 (CR 202.3f), and the card is each of the three colors (CR 202.2d).
#[test]
fn mana_value_six_and_three_colors() {
    let cost = table().get(table().find("Probe Severance").unwrap()).cost;
    assert_eq!(cost.mana_value(), 6);
    assert_eq!(cost.colors(), ColorSet::NONE.with(Color::White).with(Color::Black).with(Color::Green));
}
