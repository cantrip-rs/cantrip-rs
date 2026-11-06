//! Demonic Hordes's "sacrifice a land of an opponent's choice" with a land that can't be sacrificed (CR 101.2, 701.21a): the
//! opponent is not offered it - "you can't try to sacrifice [it], fail, and therefore ignore the effect" (Assault Suit's
//! ruling) - so beside a Forest the Forest is the one sacrificed, and the Hordes's controller does not keep the land they
//! would rather lose. No printed card in the engine makes a land that can't be sacrificed, so a test land does.

use std::sync::OnceLock;

use cantrip_engine::*;

/// "This land can't be sacrificed."
static MOOR: CardDef = CardDef {
    name: "Probe Moor",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    keywords: Keywords::of(&[Keyword::CantBeSacrificed]),
    ..CardDef::BLANK
};

/// "{T}: Add {B}." - five of them pay for Demonic Hordes.
static FONT: CardDef = CardDef {
    name: "Probe Font",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::B), "{T}: Add {B}.")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MOOR, &FONT])))
}

/// Bot1 plays Probe Moor on turn 1 and a Forest on turn 3, casts five Fonts on turn 1 and Demonic Hordes on turn 3, and at
/// the turn 5 upkeep does not pay for it. Bot2 chooses the first land offered. The game, and each choice of a land
/// Bot2 was asked, by the lands' names.
fn play() -> (Game, Vec<Vec<String>>) {
    let find = |n: &str| table().find(n).unwrap_or_else(|| panic!("no card {n}"));
    let order = ["Probe Moor", "Forest", "Probe Font", "Probe Font", "Probe Font", "Probe Font", "Probe Font", "Demonic Hordes"];
    let mut deck: Vec<CardId> = order.iter().map(|n| find(n)).collect();
    deck.extend(std::iter::repeat(find("Island")).take(30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![find("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut plays = order.iter().enumerate().peekable();
    let mut asked = Vec::new();
    let mut declined = false;
    for _ in 0..3000 {
        game.run();
        if declined && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            return (game, asked);
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.stack.is_empty() => {
                // The Moor and the Fonts on turn 1, the Forest and the Hordes on turn 3.
                let next = plays.peek().filter(|&&(i, _)| (i != 1 && i != 7) || game.state.turn.number >= 3);
                let kind = |i: usize| if i < 2 { ActionKind::PlayLand } else { ActionKind::CastSpell };
                match next.and_then(|&(i, &n)| options.iter().find(|o| o.kind == kind(i) && o.source.is_some_and(|s| game.name(s) == n))) {
                    Some(o) => {
                        plays.next();
                        Answer::Act(Action::of(o))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, mana_abilities, .. } => {
                let find = |kind| options.iter().find(|o| o.kind == kind);
                // Mana abilities precede payment. ConfirmCasting closes that window even when the pool is empty.
                let pay = if *mana_abilities {
                    find(ActionKind::ActivateAbility).or(find(ActionKind::ConfirmCasting))
                } else {
                    find(ActionKind::AssignMana).or(find(ActionKind::ConfirmCasting))
                };
                Answer::Act(Action::of(pay.unwrap_or_else(|| panic!("nothing pays: {options:?}"))))
            }
            RequestKind::UnlessPays { options, .. } => {
                declined = true;
                Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::DeclineCost).expect("a way to decline")))
            }
            RequestKind::ChooseCards { cards, rule, .. } if request.player == PlayerId(1) => {
                let choosable: Vec<ObjectId> = rule.among.clone().unwrap_or_else(|| cards.clone());
                asked.push(choosable.iter().map(|&id| game.name(id).to_string()).collect());
                Answer::Cards(choosable.into_iter().take(1).collect())
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

/// With the Moor not an option, the Forest is the only land the opponent could choose: nothing is asked, the Forest is
/// sacrificed, and the Moor stays.
#[test]
fn the_opponent_is_not_offered_a_land_that_cant_be_sacrificed() {
    let (game, asked) = play();
    let on_battlefield = |name: &str| game.state.battlefield().iter().any(|&id| game.name(id) == name);
    let in_graveyard = game.state.zone(Zone::graveyard(PlayerId(0))).iter().any(|&id| game.name(id) == "Forest");
    assert!(asked.iter().all(|a| !a.contains(&"Probe Moor".to_string())), "the Moor was offered: {asked:?}");
    assert!(on_battlefield("Probe Moor"), "the Moor stays");
    assert!(!on_battlefield("Forest") && in_graveyard, "the Forest was sacrificed");
    assert!(game.state.battlefield().iter().any(|&id| game.name(id) == "Demonic Hordes" && game.state.obj(id).tapped), "the Hordes is tapped");
}
