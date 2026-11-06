//! A reduction by hybrid mana (CR 118.7e): the payer chooses one half of each of its hybrid symbols as it is applied. No
//! printed card in the engine reduces a cost by hybrid mana yet (the power-up cards whose own cost has one would); a test
//! permanent makes spells cost {U/R} less. Probe Shock ({2}{R}) is cast with Moxen: taking {U}, which the cost has none of,
//! takes {1} instead (CR 118.7b) and {1}{R} is paid; taking {R}, {2} is paid.

use std::sync::OnceLock;

use cantrip_engine::*;

fn less_u_or_r(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(_) => CostDelta::less_mana(ManaCost { hybrid: [Some((Mana::U, Mana::R)), None, None, None, None, None], ..ManaCost::ZERO }),
        _ => CostDelta::NONE,
    }
}

static REDUCER: CardDef = CardDef { name: "Probe Hybrid Reducer", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[less_u_or_r], ..CardDef::BLANK };
static SHOCK: CardDef = CardDef {
    name: "Probe Shock",
    cost: ManaCost { generic: 2, symbols: [0, 0, 0, 1, 0, 0], ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REDUCER, &SHOCK])))
}

/// Bot1 casts Probe Shock under the reducer, answering the split with `choose`: the question's options, and the total cost.
fn split(choose: Vec<u8>) -> (Vec<String>, String) {
    let find = |name: &str| table().find(name).unwrap_or_else(|| panic!("no card {name}"));
    let mut deck = vec![find("Probe Shock")];
    deck.extend(vec![find("Swamp"); 29]);
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![find("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for name in ["Probe Hybrid Reducer", "Mox Ruby", "Mox Ruby", "Mox Ruby"] {
        let id = game.state.push_obj(Obj::new(find(name), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    let mut options = Vec::new();
    let mut total = String::new();
    let mut cast = false;
    for _ in 0..200 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        if cast && game.state.stack.is_empty() {
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && !cast && game.state.turn.step() == Step::PrecombatMain => {
                cast = true;
                Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::CastSpell).expect("Probe Shock is offered")))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseModes { modes, additional_cost: true, .. } => {
                options = modes.iter().map(|m| label_text(m).to_string()).collect();
                Answer::Modes(choose.clone())
            }
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, total: owed, remaining, mana_abilities, .. } => {
                if total.is_empty() {
                    total = owed.to_string();
                }
                super::pay(options, remaining, *mana_abilities, options.iter().find(|o| o.kind == ActionKind::ActivateAbility))
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(cast);
    (options, total)
}

#[test]
fn a_reduction_by_hybrid_mana_is_split_by_the_payer() {
    let (options, total) = split(vec![]);
    assert_eq!(options, ["Reduce by {R} for {U/R}"]);
    assert_eq!(total, "{1}{R}", "{{U}} taken");
    let (_, total) = split(vec![0]);
    assert_eq!(total, "{2}", "{{R}} taken");
}
