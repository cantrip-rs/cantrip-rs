//! Demonic Pact with only modes that need a target left, and no legal target for them (ruling 2015-06-22: the ability is
//! removed from the stack and nothing happens; CR 603.3c, 700.2). Its two targeted modes take any target and target opponent,
//! always legal while an opponent is in the game - unless its controller has shroud, the opponent hexproof, and nothing else is
//! there to target; and its "you lose the game" can be spent only by a player who can't lose. Test enchantments give that,
//! beside the printed Demonic Pact and every other card the engine has.

use std::sync::OnceLock;

use cantrip_engine::*;

/// "You have shroud." (as True Believer's, on an enchantment)
static SHROUD: CardDef = CardDef { name: "Probe Shroud", types: TypeSet::of(&[CardType::Enchantment]), player_keywords: &[cantrip_mechanics::statics::YOU_HAVE_SHROUD], ..CardDef::BLANK };
/// "You have hexproof." (as Crystal Barricade's, on an enchantment)
static HEXPROOF: CardDef = CardDef { name: "Probe Hexproof", types: TypeSet::of(&[CardType::Enchantment]), player_keywords: &[cantrip_mechanics::statics::YOU_HAVE_HEXPROOF], ..CardDef::BLANK };
/// "You can't lose the game." (as Platinum Angel's, on an enchantment)
static ANGEL: CardDef = CardDef { name: "Probe Angel", types: TypeSet::of(&[CardType::Enchantment]), loss_rules: &[cantrip_mechanics::statics::you_cant_lose_the_game], ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SHROUD, &HEXPROOF, &ANGEL])))
}

/// Turn 1: Bot1 casts four Moxen, Demonic Pact, Probe Angel and Probe Shroud. Turn 2: Bot2 casts Probe Hexproof. Turn 3: the
/// Pact offers only "draw two cards" and "you lose the game" (no target for the other two), and Bot1 draws. Turn 5: only "you lose
/// the game" is left, chosen without asking, and Bot1 can't lose. Turn 7: only the two targeted modes are left, neither with a
/// legal target: Bot1 is asked nothing, nothing happens, and the game goes on.
#[test]
fn demonic_pact_with_only_targeted_modes_left_and_no_target_does_nothing() {
    let names = |list: &[&str]| -> Vec<CardId> { list.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect() };
    let mut first = names(&["Mox Jet", "Mox Jet", "Mox Ruby", "Mox Pearl", "Demonic Pact", "Probe Angel", "Probe Shroud"]);
    first.extend(names(&["Island"; 23]));
    let mut second = names(&["Probe Hexproof"]);
    second.extend(names(&["Island"; 29]));
    let mut game = Game::new(table(), &Setup { decks: vec![first, second], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let plan: [(u32, usize, &str); 8] = [
        (1, 0, "Mox Jet"),
        (1, 0, "Mox Jet"),
        (1, 0, "Mox Ruby"),
        (1, 0, "Mox Pearl"),
        (1, 0, "Demonic Pact"),
        (1, 0, "Probe Angel"),
        (1, 0, "Probe Shroud"),
        (2, 1, "Probe Hexproof"),
    ];
    let mut steps = plan.iter().peekable();
    let mut asked: Vec<(u32, Vec<u8>)> = Vec::new();
    while game.state.turn.number < 8 {
        game.run();
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if game.state.stack.is_empty() => match steps.peek() {
                Some(&&(turn, seat, name)) if game.state.turn.number >= turn && request.player.0 as usize == seat => {
                    match options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)) {
                        Some(option) => {
                            steps.next();
                            Answer::Act(Action::of(option))
                        }
                        None => Answer::Pass,
                    }
                }
                _ => Answer::Pass,
            },
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).text().starts_with("Mox")));
                let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting);
                Answer::Act(Action::of(confirm.or(mox).expect("a Mox to pay with, or the payment to confirm")))
            }
            RequestKind::ChooseModes { eligible, .. } => {
                asked.push((game.state.turn.number, eligible.clone()));
                Answer::Modes(vec![eligible[0]])
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().rev().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(steps.peek().is_none(), "the plan was not played out");
    // The positive control: on turn 3 the modes with no target were not offered, and draw two cards was chosen.
    assert_eq!(asked, [(3, vec![2, 3])], "Bot1 was asked for a mode other than on turn 3, or offered one with no target");
    assert!(!game.state.player(PlayerId(0)).lost, "Bot1 lost");
    assert!(!game.state.stack.iter().any(|i| game.name(i.object) == "Demonic Pact"), "the Pact's ability is still on the stack");
}
