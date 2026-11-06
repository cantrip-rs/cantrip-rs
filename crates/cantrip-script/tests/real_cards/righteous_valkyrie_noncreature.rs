//! Righteous Valkyrie's "you gain life equal to that creature's toughness" (CR 208.3, 608.2h) for an Angel that stops being a
//! creature while the trigger waits on the stack: a permanent that is not a creature has no toughness, whatever the engine keeps
//! of its printed toughness, so nothing is gained. No printed card in the engine does this at instant speed (Imprisoned in the
//! Moon is not in it), so test cards do it beside the printed Righteous Valkyrie.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// "Target creature becomes an artifact and loses its other card types."
fn unlife(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::Indefinite);
    }
    Ok(())
}

static UNLIFE: CardDef = CardDef { name: "Probe Unlife", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature)], unlife), ..CardDef::BLANK };
/// A 1/3 Angel that costs nothing.
static ANGEL: CardDef = CardDef {
    name: "Probe Angel",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Angel]),
    power: Some(1),
    toughness: Some(3),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&UNLIFE, &ANGEL])))
}

/// Bot1 casts Righteous Valkyrie off three Moxen, then the Angel; with the Valkyrie's trigger for it on the stack, `respond`
/// says whether Bot1 turns the Angel into an artifact. Bot1's life once everything has resolved.
fn life_after(respond: bool) -> i32 {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let deck = |hand: &[&str]| {
        let mut deck = names(hand);
        deck.extend(names(&vec!["Island"; 30 - hand.len()]));
        deck
    };
    let mut hand = vec!["Mox Pearl", "Mox Pearl", "Mox Pearl", "Righteous Valkyrie", "Probe Angel"];
    if respond {
        hand.push("Probe Unlife");
    }
    let mut game = Game::new(table(), &Setup { decks: vec![deck(&hand), deck(&[])], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = hand.clone().into_iter().peekable();
    for _ in 0..3000 {
        game.run();
        if casts.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            return game.state.player(PlayerId(0)).life;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                // The response waits until the Angel is on the battlefield, with the Valkyrie's trigger on the stack.
                let ready = |name: &str| name != "Probe Unlife" || game.state.battlefield().iter().any(|&id| game.name(id) == "Probe Angel");
                let option = casts.peek().filter(|&&name| ready(name)).and_then(|&name| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|c| game.name(c) == name)));
                match option {
                    Some(option) => {
                        casts.next();
                        let mut action = Action::of(option);
                        if option.targets_required > 0 {
                            let angel = *game.state.battlefield().iter().find(|&&id| game.name(id) == "Probe Angel").expect("the Angel");
                            action.targets = vec![Target::Object(angel)];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility);
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().rev().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

/// Control: the Angel stays a 1/3 creature, so the Valkyrie's trigger gains its toughness, 3.
#[test]
fn an_angel_that_stays_a_creature_gains_its_toughness() {
    assert_eq!(life_after(false), 23);
}

/// The Angel becomes a noncreature artifact in response (CR 205.1a): it has no toughness (CR 208.3) as the ability resolves,
/// so nothing is gained, though the engine keeps the printed 3.
#[test]
fn an_angel_that_is_no_creature_as_the_trigger_resolves_gains_nothing() {
    assert_eq!(life_after(true), 20);
}
