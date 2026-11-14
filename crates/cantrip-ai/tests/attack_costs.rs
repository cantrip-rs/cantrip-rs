//! A cost to attack is not a requirement (CR 508.1d): a creature that attacks each combat if able need not attack a
//! player whom attacking costs something (Ghostly Prison), and a seat that cannot pay may leave it home. The AI player's
//! factoring of a declaration of attackers must offer that, or a seat with no mana is made to attack, cannot pay, has the
//! declaration taken back (CR 508.1h-j, `turn.rs`) and is asked again - the same answer forever.

use cantrip_ai::eval::{Evaluator, FeatureBatch, Outputs};
use cantrip_ai::net::{self, Choice};
use cantrip_ai::record::table;
use cantrip_ai::{EvalError, Knowledge};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_script::GameScript;

/// Every option alike: the first legal one is taken at every step.
struct Uniform;

impl Evaluator for Uniform {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs {
            value: vec![0.0; b.rows],
            policy: b.options.iter().map(|&(_, n)| vec![0.0; n]).collect(),
            aux: vec![[0.0; 2]; b.rows],
        })
    }
}

/// The last legal option taken at every step: in a declaration of attackers, every creature attacking.
struct Eager;

impl Evaluator for Eager {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        Ok(Outputs {
            value: vec![0.0; b.rows],
            policy: b.options.iter().map(|&(_, n)| (0..n).map(|i| i as f32).collect()).collect(),
            aux: vec![[0.0; 2]; b.rows],
        })
    }
}

/// Bot1 has Anje's Ravager ("attacks each combat if able") and no mana left; Bot2 has Ghostly Prison, so attacking Bot2
/// costs {2} and nothing else may be attacked.
const SCRIPT: &str = "\
[Bot1 Deck]
Black Lotus
Anje's Ravager
Island x16

[Bot2 Deck]
Plains
Ghostly Prison
Mox Pearl
Mox Jet
Plains x14

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Anje's Ravager.Cast
[T2 Bot2.Main1]
Bot2.Hand.Plains.Play
Bot2.Hand.Mox Pearl.Cast
Bot2.Hand.Mox Jet.Cast
Bot2.Battlefield.Plains.Activate
Bot2.Battlefield.Mox Pearl.Activate
Bot2.Battlefield.Mox Jet.Activate
Bot2.Hand.Ghostly Prison.Cast
[T3 Bot1.Main1]
[Handover Bot1.Attack]

[Expect]
Turn = 3
Step = Attackers
";

#[test]
fn a_seat_that_cannot_pay_to_attack_leaves_a_must_attack_creature_home() {
    let script = GameScript::parse(SCRIPT, "attack-cost-no-mana").expect("the script parses");
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    assert!(result.handed_over, "handed over at the declaration of attackers");
    let mut game = result.game;
    let request = game.pending().cloned().expect("a declaration of attackers");
    let RequestKind::DeclareAttackers { costs, requirements, .. } = &request.kind else {
        panic!("not a declaration of attackers: {request:?}")
    };
    assert!(!costs.is_empty() && !requirements.required.is_empty() && requirements.most == 0);
    // The seats answer as the AI player does until Bot1's turn is over; a declaration taken back again and again never
    // gets there.
    let mut declarations = 0;
    for _ in 0..200 {
        if game.state.turn.number > 3 {
            break;
        }
        let request = game.pending().cloned().expect("a request");
        if matches!(request.kind, RequestKind::DeclareAttackers { .. }) {
            declarations += 1;
        }
        let knowledge = Knowledge::of(&game, request.player);
        let decision = net::answer(&game, &request, &knowledge, InfoMode::Full, table(), &Uniform, Choice::Best).expect("an answer");
        game.answer(request.id, request.player, decision.answer).expect("the answer is taken");
        game.run();
    }
    assert!(game.state.turn.number > 3, "Bot1's turn never ended: {declarations} declarations of attackers");
    assert_eq!(declarations, 1);
}

/// Bot2 has Flooded Woodlands ("Green creatures can't attack unless their controller sacrifices a land of their choice for
/// each green creature they control that's attacking"); Bot1 has three Llanowar Elves able to attack and two lands. All
/// three attacking can't be paid (CR 508.1j, 118.3): the payment fails with nothing asked, so a seat that declared it
/// and was asked again would declare it forever - the validator refuses it instead. Both libraries are short: the game ends by a draw from an empty one.
const WOODLANDS: &str = "\
[Bot1 Deck]
Forest
Black Lotus
Llanowar Elves
Llanowar Elves
Llanowar Elves
Forest
Forest
Forest x5

[Bot2 Deck]
Island
Black Lotus
Mox Jet
Flooded Woodlands
Island
Island
Island
Island x5

[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Hand.Llanowar Elves.Cast
Bot1.Hand.Llanowar Elves.Cast
Bot1.Hand.Llanowar Elves.Cast
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Hand.Mox Jet.Cast
Bot2.Battlefield.Black Lotus.Activate(Color: Blue)
Bot2.Battlefield.Mox Jet.Activate
Bot2.Hand.Flooded Woodlands.Cast
[T3 Bot1.Main1]
Bot1.Hand.Forest.Play
[Handover Bot1.Attack]

[Expect]
Turn = 3
Step = Attackers
";

#[test]
fn ai_seats_facing_flooded_woodlands_with_more_elves_than_lands_play_the_game_to_its_end() {
    let script = GameScript::parse(WOODLANDS, "flooded-woodlands-short-of-lands").expect("the script parses");
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    assert!(result.handed_over, "handed over at the declaration of attackers");
    let game = result.game;
    let request = game.pending().cloned().expect("a declaration of attackers");
    let RequestKind::DeclareAttackers { eligible, requirements, .. } = &request.kind else {
        panic!("not a declaration of attackers: {request:?}")
    };
    let all: Vec<(ObjectId, Target)> = eligible.iter().map(|&elf| (elf, Target::Player(PlayerId(1)))).collect();
    assert_eq!(all.len(), 3);
    assert!(requirements.unpaid(&all).is_some(), "three Elves can't all be paid for with two lands");
    play_to_the_end(game);
}

/// Every seat answers as the AI player does until the game is over, attacking with everything it may; a declaration taken
/// back and asked again would be asked again in the same turn, and the game would never end. The permanents to sacrifice
/// are the first ones offered, an enlist taps the first creature offered, and an order of payment is the first one offered
/// (the default, `ChooseCostPaymentPromptMessage.Default`): the factoring's first option at a sacrifice is to give the
/// payment up, which takes the declaration back for the seat to make again - a seat that always gives up is one that never
/// attacks, as with a mana payment, a seat's choice and not the rules'.
fn play_to_the_end(mut game: Game) {
    let mut declarations: Vec<u32> = Vec::new();
    let mut attacked = 0;
    for _ in 0..20_000 {
        if game.is_over() {
            break;
        }
        let request = game.pending().cloned().expect("a request");
        if matches!(request.kind, RequestKind::DeclareAttackers { .. }) {
            declarations.push(game.state.turn.number);
        }
        let knowledge = Knowledge::of(&game, request.player);
        let given = match &request.kind {
            RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, cards, rule, .. } => {
                let Count::Exactly(n) = rule.count else { panic!("{:?}", rule.count) };
                Some(Answer::Cards(cards[..n as usize].to_vec()))
            }
            RequestKind::ChooseCards { purpose: Purpose::Enlist, cards, .. } => Some(Answer::Cards(cards[..1].to_vec())),
            RequestKind::ChooseCostPayment { .. } => Some(Answer::CostPayment(0)),
            _ => None,
        };
        if let Some(answer) = given {
            game.answer(request.id, request.player, answer).expect("the answer is taken");
            game.run();
            continue;
        }
        let decision = if matches!(request.kind, RequestKind::DeclareAttackers { .. }) {
            net::answer(&game, &request, &knowledge, InfoMode::Full, table(), &Eager, Choice::Best)
        } else {
            net::answer(&game, &request, &knowledge, InfoMode::Full, table(), &Uniform, Choice::Best)
        }
        .expect("an answer");
        if let Answer::Attack(attacks) = &decision.answer
            && let RequestKind::DeclareAttackers { requirements, .. } = &request.kind
        {
            attacked += attacks.len();
            assert!(requirements.unpaid(attacks).is_none(), "the AI declared attacks it can't pay for: {attacks:?}");
        }
        game.answer(request.id, request.player, decision.answer).expect("the answer is taken");
        game.run();
    }
    assert!(attacked > 0, "nobody attacked: the cost to attack was never met");
    assert!(game.is_over(), "the game never ended: declarations of attackers in turns {declarations:?}");
    let mut turns = declarations.clone();
    turns.dedup();
    assert_eq!(turns, declarations, "a declaration of attackers was asked again within a turn: {declarations:?}");
}

/// Llanowar Elves and Coalition Skyknight attack into Bot2's Flooded Woodlands with Dryad Arbor Bot1's only land: the
/// Skyknight enlists the Arbor, which is then sacrificed for the Elves (CR 702.154a, 508.1j). Sacrificed first, the Arbor
/// couldn't be tapped and the payment would be taken back unasked; a seat taking the first order offered each time would
/// make the same declaration forever. Short libraries end the game.
const SKYKNIGHT: &str = "[Bot1 Deck]
Dryad Arbor
Black Lotus
Black Lotus
Llanowar Elves
Coalition Skyknight
Island
Island
Island x5

[Bot2 Deck]
Island
Black Lotus
Mox Jet
Flooded Woodlands
Island
Island
Island
Island x5

[Actions]
Bot1.Hand.Dryad Arbor.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus#2.Activate(Color: White)
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Hand.Llanowar Elves.Cast
Bot1.Hand.Coalition Skyknight.Cast
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Hand.Mox Jet.Cast
Bot2.Battlefield.Black Lotus.Activate(Color: Blue)
Bot2.Battlefield.Mox Jet.Activate
Bot2.Hand.Flooded Woodlands.Cast
[T3 Bot1.Main1]
[Handover Bot1.Attack]

[Expect]
Turn = 3
Step = Attackers
";

#[test]
fn ai_seats_with_an_enlisted_dryad_arbor_under_flooded_woodlands_play_the_game_to_its_end() {
    let script = GameScript::parse(SKYKNIGHT, "flooded-woodlands-enlisted-dryad-arbor").expect("the script parses");
    let result = cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}
{}", f.message, f.report));
    assert!(result.handed_over, "handed over at the declaration of attackers");
    let mut game = result.game;
    // The first turn's declaration: both attack, the Skyknight enlists the Arbor, and it is tapped, then sacrificed.
    let arbor = game.state.battlefield().iter().copied().find(|&id| game.chars(id).name == "Dryad Arbor").expect("the Arbor");
    let request = game.pending().cloned().expect("a declaration of attackers");
    let RequestKind::DeclareAttackers { eligible, .. } = &request.kind else { panic!("{request:?}") };
    let both: Vec<(ObjectId, Target)> = eligible.iter().filter(|&&id| id != arbor).map(|&id| (id, Target::Player(PlayerId(1)))).collect();
    assert_eq!(both.len(), 2);
    game.answer(request.id, request.player, Answer::Attack(both)).expect("payable: tap the Arbor, then sacrifice it");
    game.run();
    for _ in 0..4 {
        let request = game.pending().cloned().expect("a request");
        let answer = match &request.kind {
            RequestKind::ChooseCards { purpose: Purpose::Enlist, .. } => Answer::Cards(vec![arbor]),
            RequestKind::ChooseCostPayment { .. } => Answer::CostPayment(0),
            RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, cards, .. } => Answer::Cards(cards.clone()),
            _ => break,
        };
        game.answer(request.id, request.player, answer).expect("the answer is taken");
        game.run();
    }
    assert_eq!(game.state.attackers.len(), 2, "both attack, paid for with the Arbor");
    play_to_the_end(game);
}
