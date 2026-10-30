//! Static abilities that forbid some payments of the costs to cast spells and activate abilities (`CardDef::cost_bans`, CR
//! 101.2, 118.3, 601.2h, 602.2b), on a table of test-only cards. A cost that can only be paid by a forbidden payment can't be
//! paid, so its ability is not offered; one with an allowed way (a land for "sacrifice a land") still is; a mana ability is
//! an ability too, unless the ban leaves mana abilities out (Karn's Sylex); and what a resolving ability has a player pay is
//! untouched. The printed case is Yasharn, Implacable Earth (`scenarios/yasharn-*.cantrip`).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn land(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Land)
}

/// "Players can't pay life or sacrifice nonland permanents to cast spells or activate abilities" (Yasharn's).
fn no_life_or_nonland_sacrifices(q: &dyn Query, _: &Asker, _: PlayerId, _: CostFor, payment: CostPayment) -> bool {
    match payment {
        CostPayment::Life => true,
        CostPayment::Sacrifice(id) => !q.chars(id).is(CardType::Land),
        CostPayment::BeamUp(_) => false,
    }
}

/// "Players can't pay life to cast spells or to activate abilities that aren't mana abilities" (Karn's Sylex's).
fn no_life_but_for_mana_abilities(_: &dyn Query, _: &Asker, _: PlayerId, paid_for: CostFor, payment: CostPayment) -> bool {
    payment == CostPayment::Life && paid_for != CostFor::ManaAbility
}

static BAN: CardDef = CardDef {
    name: "Probe Ban",
    types: TypeSet::of(&[CardType::Artifact]),
    cost_bans: &[no_life_or_nonland_sacrifices],
    ..CardDef::BLANK
};

static SYLEX: CardDef = CardDef {
    name: "Probe Sylex",
    types: TypeSet::of(&[CardType::Artifact]),
    cost_bans: &[no_life_but_for_mana_abilities],
    ..CardDef::BLANK
};

const PAY: Question = Question::new("PayLife", "pay {cost}?", "", "Pay {cost}", "Don't");

/// "You may pay 1 life and sacrifice a creature. If you do, you lose 5 life": a cost paid as the ability resolves (CR 118.12).
fn pay_as_it_resolves(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let cost = Cost { life: 1, ..Cost::sacrifice(1, creature) };
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY)? {
        g.lose_life(item.controller, 5);
    }
    Ok(())
}

static COSTS: CardDef = CardDef {
    name: "Probe Costs",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(life(1), nothing, "Pay 1 life: Nothing happens."),
        ability(Cost::sacrifice(1, creature), nothing, "Sacrifice a creature: Nothing happens."),
        ability(Cost::sacrifice(1, land), nothing, "Sacrifice a land: Nothing happens."),
        ability(Cost { sacrifice_self: true, ..Cost::NONE }, nothing, "Sacrifice this artifact: Nothing happens."),
        ability(cost("{T}"), pay_as_it_resolves, "{T}: You may pay 1 life and sacrifice a creature. If you do, you lose 5 life."),
        ability(cost("{U/P}"), nothing, "{U/P}: Nothing happens."),
    ],
    ..CardDef::BLANK
};

/// A special action (CR 116.2) that costs {U/P}: no activation, so a ban on paying life to activate abilities leaves it.
static RITE: CardDef = CardDef {
    name: "Probe Rite",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[AbilityDef::Activated(ActivatedDef { text: "{U/P}: Nothing happens. (A special action.)", cost: cost("{U/P}"), effect: nothing, special_action: true, ..ActivatedDef::BLANK })],
    ..CardDef::BLANK
};

/// "{T}: Add {U}."
static BLUE: CardDef = CardDef {
    name: "Probe Blue",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")],
    ..CardDef::BLANK
};

/// "As an additional cost to cast this spell, sacrifice one or more creatures": a variable sacrifice (`VariableCost`).
static OFFERING: CardDef = CardDef {
    name: "Probe Offering",
    types: TypeSet::of(&[CardType::Sorcery]),
    additional_cost: Some(Cost { variable: Some(VariableCost::SacrificeOneOrMore(creature)), ..Cost::NONE }),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};

static WELL: CardDef = CardDef {
    name: "Probe Well",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(life(1), add(Mana::C), "Pay 1 life: Add {C}.")],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&BAN, &SYLEX, &COSTS, &RITE, &BLUE, &OFFERING, &WELL, &BEAR, &FILLER]))
}

/// The board every test starts from: Bot1 plays a land and casts a Bear, Probe Costs and Probe Well, and `ban` first if
/// it is named.
fn setup(ban: Option<&str>) -> Vec<(ActionKind, &'static str, u8)> {
    let mut steps = vec![(ActionKind::PlayLand, "Filler", 0)];
    if let Some(ban) = ban {
        steps.push((ActionKind::CastSpell, if ban == "Probe Ban" { "Probe Ban" } else { "Probe Sylex" }, 0));
    }
    steps.extend([(ActionKind::CastSpell, "Probe Bear", 0), (ActionKind::CastSpell, "Probe Costs", 0), (ActionKind::CastSpell, "Probe Well", 0), (ActionKind::CastSpell, "Probe Rite", 0)]);
    steps
}

/// Plays Bot1's turn 1: each (kind, name, index of the cast or ability) in order, as it is offered; everything else passes.
/// A cost's choice of permanents takes the first, a payment asked as an ability resolves is paid, and Phyrexian symbols are
/// paid with life where they may be. The game.
fn play(steps: &[(ActionKind, &str, u8)]) -> Game {
    play_holding(steps, None)
}

/// `play`, with `held` in the opening hand as well, never cast.
fn play_holding(steps: &[(ActionKind, &str, u8)], held: Option<&str>) -> Game {
    let filler = table().find("Filler").unwrap();
    // The opening hand: each card a step plays or casts, in order, and the card held.
    let mut deck: Vec<CardId> = held.map(|n| table().find(n).unwrap()).into_iter().collect();
    for &(kind, name, _) in steps {
        let card = table().find(name).unwrap();
        if matches!(kind, ActionKind::PlayLand | ActionKind::CastSpell) && !deck.contains(&card) {
            deck.push(card);
        }
    }
    assert!(deck.len() <= 7, "the opening hand holds every card cast");
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let others = vec![filler; 30];
    let mut game = Game::new(table(), &Setup { decks: vec![deck, others], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut todo = steps.iter().peekable();
    for _ in 0..200 {
        game.run();
        // Done once every step is taken and resolved, and nothing it began is still asking.
        let asking = game.pending().is_some_and(|r| !matches!(r.kind, RequestKind::Priority { .. }));
        if todo.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() && !asking {
            break;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let option = todo.peek().and_then(|&&(kind, name, index)| {
                    options.iter().find(|o| o.kind == kind && o.index == index && o.source.is_some_and(|s| game.name(s) == name))
                });
                match option {
                    Some(option) => {
                        todo.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCards { cards, rule, .. } => {
                let eligible = rule.among.clone().unwrap_or_else(|| cards.clone());
                Answer::Cards(eligible.into_iter().take(1).collect())
            }
            // Every Phyrexian symbol it may is paid with life.
            RequestKind::ChooseModes { eligible, at_most, .. } => Answer::Modes(eligible.iter().copied().take(usize::from(*at_most)).collect()),
            // Mana is paid from the pool, or by tapping for it.
            RequestKind::PayCosts { options, .. } => match options.iter().find(|o| o.kind == ActionKind::AssignMana).or_else(|| options.iter().find(|o| o.kind == ActionKind::ActivateAbility)) {
                Some(pay) => Answer::Act(Action::of(pay)),
                None => panic!("nothing pays the cost: {options:?}"),
            },
            RequestKind::UnlessPays { options, .. } => match options.iter().find(|o| o.kind == ActionKind::PayCost) {
                Some(pay) => Answer::Act(Action::of(pay)),
                None => panic!("the payment as the ability resolves is not offered"),
            },
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(todo.peek().is_none(), "every step was offered");
    game
}

/// The abilities of `name` Bot1 is offered at its next priority.
fn offered(game: &mut Game, name: &str) -> Vec<u8> {
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    options.iter().filter(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == name)).map(|o| o.index).collect()
}

/// The positive control: with no ban, every cost is payable - the life, the creature, the land, the artifact itself, a
/// Phyrexian symbol with life, there being no blue mana - and the mana ability that pays life is offered.
#[test]
fn with_no_ban_every_cost_is_offered() {
    let mut game = play(&setup(None));
    assert_eq!(offered(&mut game, "Probe Costs"), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(offered(&mut game, "Probe Well"), vec![0]);
}

/// CR 101.2, 602.2b: under "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities", an
/// ability that pays life, sacrifices a creature or sacrifices its nonland self is not offered, nor is a mana ability that
/// pays life, nor one whose {U/P} only life could pay; one that sacrifices a land still is, and so is the one whose payment
/// comes as it resolves.
#[test]
fn a_ban_refuses_life_and_nonland_sacrifices_but_not_a_land() {
    let mut game = play(&setup(Some("Probe Ban")));
    assert_eq!(offered(&mut game, "Probe Costs"), vec![2, 4]);
    assert_eq!(offered(&mut game, "Probe Well"), Vec::<u8>::new());
}

/// The land is sacrificed for "Sacrifice a land" under the ban: the cost is paid, not only offered.
#[test]
fn a_land_is_sacrificed_under_the_ban() {
    let mut steps = setup(Some("Probe Ban"));
    steps.push((ActionKind::ActivateAbility, "Probe Costs", 2));
    let game = play(&steps);
    let filler = table().find("Filler").unwrap();
    assert!(game.state.zone(Zone::graveyard(PlayerId(0))).iter().any(|&c| game.state.obj(c).card == filler));
}

/// CR 118.12 and Yasharn's ruling ("Other things may still cause players to pay life or sacrifice creatures, such as a
/// resolving spell or ability"): under the ban, the life and the creature an ability's effect asks for are paid as it
/// resolves - 1 life, then the 5 the payment brings, and the Bear goes to the graveyard.
#[test]
fn a_payment_as_an_ability_resolves_is_untouched() {
    let mut steps = setup(Some("Probe Ban"));
    steps.push((ActionKind::ActivateAbility, "Probe Costs", 4));
    let game = play(&steps);
    assert_eq!(game.state.player(PlayerId(0)).life, 14);
    let bear = table().find("Probe Bear").unwrap();
    assert!(game.state.zone(Zone::graveyard(PlayerId(0))).iter().any(|&c| game.state.obj(c).card == bear));
}

/// A ban that leaves mana abilities out ("abilities that aren't mana abilities", Karn's Sylex) refuses the ability that
/// pays life (with its {U/P} too), offers the mana ability that does, and leaves sacrifices alone.
#[test]
fn a_ban_on_life_for_abilities_that_arent_mana_abilities_leaves_mana_abilities() {
    let mut game = play(&setup(Some("Probe Sylex")));
    assert_eq!(offered(&mut game, "Probe Costs"), vec![1, 2, 3, 4]);
    assert_eq!(offered(&mut game, "Probe Well"), vec![0]);
}

/// CR 116.2: a special action is no activated ability, so a ban on paying life to activate abilities does not reach it:
/// under the ban, with no blue mana, its {U/P} is paid with 2 life.
#[test]
fn a_special_action_pays_life_under_the_ban() {
    let mut steps = setup(Some("Probe Ban"));
    steps.push((ActionKind::ActivateAbility, "Probe Rite", 0));
    let game = play(&steps);
    assert_eq!(game.state.player(PlayerId(0)).life, 18);
}


/// Whether Bot1 is offered to cast `name` at its next priority.
fn cast_offered(game: &mut Game, name: &str) -> bool {
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name))
}

/// A variable sacrifice ("sacrifice one or more creatures", `VariableCost`) is a sacrifice like any other: under the ban
/// the Bear can't pay it and the spell is not offered; without it, it is (the positive control).
#[test]
fn a_ban_refuses_a_variable_sacrifice() {
    let mut steps = vec![(ActionKind::PlayLand, "Filler", 0), (ActionKind::CastSpell, "Probe Bear", 0)];
    let mut game = play_holding(&steps, Some("Probe Offering"));
    assert!(cast_offered(&mut game, "Probe Offering"));
    steps.insert(1, (ActionKind::CastSpell, "Probe Ban", 0));
    let mut game = play_holding(&steps, Some("Probe Offering"));
    assert!(!cast_offered(&mut game, "Probe Offering"));
}

/// CR 107.4f, 601.2b by way of 602.2b: under the ban, an ability's {U/P} with blue mana there to pay it is paid with the
/// blue, never 2 life - the announcement of life offers none.
#[test]
fn a_phyrexian_symbol_of_an_ability_is_paid_with_mana_under_the_ban() {
    let mut steps = setup(Some("Probe Ban"));
    steps.push((ActionKind::CastSpell, "Probe Blue", 0));
    steps.push((ActionKind::ActivateAbility, "Probe Costs", 5));
    let game = play(&steps);
    assert_eq!(game.state.player(PlayerId(0)).life, 20);
    let blue = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Probe Blue").expect("Probe Blue");
    assert!(game.state.obj(blue).tapped, "the {{U}} was paid with Probe Blue's mana");
}
