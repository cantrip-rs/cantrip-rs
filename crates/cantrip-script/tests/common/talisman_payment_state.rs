//! Printed-card position: Old Man of the Sea holds a blue Merfolk, and Thoughtlace made Foraging Wickermaw blue.
//! Jihad chose the Wickermaw's controller and blue. During a Talisman's payment Wickermaw can become red, making
//! Jihad's state condition true, before untapping Old Man returns the blue Merfolk. The pending state trigger
//! must survive that later false state (CR 603.8, 608.2c). The setup forces those earlier resolved effects directly.
use cantrip_engine::*;
use cantrip_state::{ChoiceRecord, Choices, Entered};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn put(game: &mut Game, name: &str, owner: PlayerId, zone: Zone) -> ObjectId {
    let card = game.cards.find(name).unwrap_or_else(|| panic!("missing {name}"));
    let mut object = Obj::new(card, owner, zone);
    object.timestamp = game.state.timestamp();
    let id = game.state.push_obj(object);
    game.state.zone_push(zone, id);
    id
}

fn changed(game: &mut Game, source: ObjectId, affected: ObjectId, kind: EffectKind, duration: Duration) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source, controller: game.state.obj(source).controller,
        timestamp, affected, kind, duration, words: Words::NONE });
}

pub fn play(talisman: &str, spell: &str, mana_color: Color, expect_sacrifice: bool) {
    let table = cards();
    let land = if spell == "Lightning Bolt" { "Mountain" } else { "Forest" };
    let mut decks = vec![vec![table.find("Island").unwrap(); 30]; 2];
    decks[0][0] = table.find(spell).unwrap();
    let mut game = Game::new(table, &Setup { decks, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let talisman_id = put(&mut game, talisman, YOU, Zone::BATTLEFIELD);
    let old_man = put(&mut game, "Old Man of the Sea", THEM, Zone::BATTLEFIELD);
    game.state.update_obj(old_man, |o| o.tapped = true);
    let merfolk = put(&mut game, "Merfolk of the Pearl Trident", YOU, Zone::BATTLEFIELD);
    game.state.update_obj(merfolk, |o| { o.controller = THEM; o.marks.control_base = Some(YOU); });
    let man_card = game.state.obj(old_man).card;
    changed(&mut game, old_man, merfolk, EffectKind::Control(THEM), Duration::AsLongAs(man_card, 0));
    let wickermaw = put(&mut game, "Foraging Wickermaw", YOU, Zone::BATTLEFIELD);
    let thoughtlace = put(&mut game, "Thoughtlace", YOU, Zone::graveyard(YOU));
    changed(&mut game, thoughtlace, wickermaw, EffectKind::SetColors(ColorSet::of(Color::Blue)), Duration::Indefinite);
    let jihad = put(&mut game, "Jihad", THEM, Zone::BATTLEFIELD);
    let jihad_card = game.state.obj(jihad).card;
    game.state.update_obj(jihad, |o| o.marks.choices = Choices::entered(jihad_card,
        Entered { color: Some(Color::Blue), player: Some(YOU), ..Default::default() }));
    // Lands are colorless: Wickermaw is the only blue permanent YOU control until Old Man untaps.
    for _ in 0..5 { put(&mut game, land, YOU, Zone::BATTLEFIELD); }
    game.state.see_statics();
    let (mut cast, mut activated_wickermaw, mut paid, mut finished) = (false, false, false, false);
    for _ in 0..500 {
        game.run();
        let request = game.pending().cloned().expect("game reaches a question");
        if cast && game.state.stack.is_empty() && game.state.triggers.is_empty()
            && matches!(request.kind, RequestKind::Priority { .. }) {
            finished = true;
            break;
        }
        let answer = match &request.kind {
            RequestKind::Priority { options } if !cast && request.player == YOU
                && game.state.turn.step() == Step::PrecombatMain => {
                assert!(game.state.battlefield().contains(&jihad), "Jihad was not protected by blue Wickermaw");
                assert_eq!(game.state.obj(merfolk).controller, THEM);
                let option = options.iter().find(|o| o.kind == ActionKind::CastSpell
                    && o.source.is_some_and(|s| game.name(s) == spell)).expect("spell is offered");
                let mut action = Action::of(option);
                action.targets = vec![if spell == "Lightning Bolt" { Target::Player(THEM) } else { Target::Object(wickermaw) }];
                cast = true;
                Answer::Act(action)
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseTarget { source, .. } if *source == talisman_id => Answer::Targets(vec![Target::Object(old_man)]),
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let option = if *mana_abilities && !remaining.is_zero() {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == land))
                } else {
                    options.iter().find(|o| o.kind == ActionKind::AssignMana)
                        .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting))
                }.expect("mana cost can be paid");
                Answer::Act(Action::of(option))
            }
            RequestKind::UnlessPays { source, options, .. } if *source == talisman_id => {
                let enough = game.state.player(YOU).pool.total() >= 3;
                let option = if enough && !activated_wickermaw {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(wickermaw))
                        .expect("Wickermaw can change color during the payment")
                } else if enough {
                    let link = game.cards.linked_to(&game.state, jihad);
                    assert_eq!(game.state.chosen_color(link), Some(Color::Blue));
                    assert_eq!(game.state.chosen_player(link), Some(YOU));
                    let blue: Vec<_> = game.state.battlefield().iter().copied().filter(|&id|
                        game.state.obj(id).controller == YOU && game.chars(id).colors.has(Color::Blue)).collect();
                    assert_eq!(blue.is_empty(), mana_color == Color::Red, "blue permanents at payment: {blue:?}");
                    assert!(game.state.obj(old_man).tapped, "Old Man must still hold Merfolk before payment");
                    paid = true;
                    options.iter().find(|o| o.kind == ActionKind::PayCost).expect("pay three")
                } else {
                    options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == land))
                        .expect("land makes the remaining mana")
                };
                let mut action = Action::of(option);
                if action.source == Some(wickermaw) { action.color = Some(mana_color); activated_wickermaw = true; }
                Answer::Act(action)
            }
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer.clone()).unwrap_or_else(|error| panic!("{error:?}, answer {answer:?}, question {:?}", request.kind));
    }
    assert!(finished && cast && paid && activated_wickermaw);
    assert!(!game.state.obj(old_man).tapped);
    assert_eq!(game.state.obj(merfolk).controller, YOU, "the later untap restores a blue permanent");
    assert_eq!(game.chars(wickermaw).colors, ColorSet::of(mana_color));
    assert_eq!(game.state.zone(Zone::graveyard(THEM)).iter().any(|&id| game.name(id) == "Jihad"), expect_sacrifice,
        "Jihad must remember the state between mana payment and untapping");
    assert_eq!(game.state.battlefield().contains(&jihad), !expect_sacrifice,
        "a sacrificed Jihad leaves its old battlefield identity behind (CR 400.7)");
}

