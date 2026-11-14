//! Ward - pay life equal to this creature's power (CR 702.21a, 118.12, 119.4) for a power above 255: the payer must have it
//! all, and pays it in one payment - one loss of life, which "whenever a player loses life" sees once
//! (`triggers::ward_pay_life`). No printed creature in the engine reaches such a power, so a test creature with Raubahn, Bull
//! of Ala Mhigo's ward does here.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// A zero-cost 300/300 with "Ward - Pay life equal to its power."
static TITAN: CardDef = CardDef {
    name: "Probe Titan",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(300),
    toughness: Some(300),
    abilities: &[cantrip_mechanics::triggers::ward_life_equal_to_power("Ward\u{2014}Pay life equal to Probe Titan's power.")],
    ..CardDef::BLANK
};

/// A zero-cost instant: target creature (nothing happens to it).
static POKE: CardDef = CardDef { name: "Probe Poke", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature)], nothing), ..CardDef::BLANK };

fn a_player_loses_life(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, negative: true, .. } => Some(Fired { subject: Target::Player(*player), data: [0, 0] }),
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost enchantment: "Whenever a player loses life, you gain 1 life."
static WITNESS: CardDef = CardDef {
    name: "Probe Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, a_player_loses_life), gain_one, "Whenever a player loses life, you gain 1 life.")],
    ..CardDef::BLANK
};

/// A land to fill hands and libraries.
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&TITAN, &POKE, &WITNESS, &FILLER]))
}

/// Bot1 casts the Titan and the Witness on turn 1; Bot2, at `life`, pokes the Titan on turn 2 and pays the ward whenever it is
/// asked to. Bot2's life afterwards, whether it was asked, whether the Poke went uncountered, and Bot1's life (20 and 1 for each
/// time the Witness saw a player lose life).
fn poke_at(life: i32) -> (i32, bool, bool, i32) {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let deck = |first: &[&str]| {
        let mut deck = names(first);
        deck.extend(names(&vec!["Filler"; 30 - first.len()]));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![deck(&["Probe Titan", "Probe Witness"]), deck(&["Probe Poke"])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game.state.update_player(PlayerId(1), |p| p.life = life);
    let (mut asked, mut poked) = (false, false);
    for _ in 0..500 {
        game.run();
        if poked && game.state.stack.is_empty() {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let named = |options: &[ActionOption], name: &str| -> Option<ActionOption> {
            options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)).cloned()
        };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => match named(options, "Probe Titan").or_else(|| named(options, "Probe Witness")) {
                Some(option) => Answer::Act(Action::of(&option)),
                None => Answer::Pass,
            },
            RequestKind::Priority { options } if game.state.turn.number == 2 && !poked && game.state.stack.is_empty() => {
                match named(options, "Probe Poke") {
                    Some(option) => {
                        poked = true;
                        let titan = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Probe Titan").expect("the Titan");
                        let mut action = Action::of(&option);
                        action.targets = vec![Target::Object(titan)];
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::UnlessPays { options, .. } => {
                asked = true;
                Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::PayCost).expect("paying is offered")))
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(poked, "Bot2 never poked the Titan");
    let resolved = game.log.iter().any(|r| matches!(r.event, Event::Countered { .. }));
    (game.state.player(PlayerId(1)).life, asked, !resolved, game.state.player(PlayerId(0)).life)
}

/// At 400 life Bot2 may pay 300: it is asked, pays it in one payment - the Witness triggers once - is left at 100, and the
/// Poke is not countered.
#[test]
fn ward_of_more_than_255_life_is_paid_in_full_in_one_payment() {
    assert_eq!(poke_at(400), (100, true, true, 21));
}

/// At 299 life Bot2 cannot pay 300 (CR 119.4): it is not asked, keeps its life, and the Poke is countered.
#[test]
fn ward_of_more_life_than_the_payer_has_counters_unasked() {
    assert_eq!(poke_at(299), (299, false, false, 20));
}
