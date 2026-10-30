//! The wire's answers as the engine's, resolving a seat's handles back to objects.
//!
//! | wire answer                                   | native `Answer`                                              |
//! |-----------------------------------------------|--------------------------------------------------------------|
//! | `PassAnswer`                                  | `Pass`                                                       |
//! | `CancelAnnouncementAnswer`                    | `Cancel`                                                     |
//! | `CardsChosenAnswer`                           | `Cards`                                                      |
//! | `TakeActionAnswer`                            | `Act` (a mana ability's `Sacrifice` becomes its target)      |
//! | `AttackersDeclaredAnswer`                     | `Attack` (a player or a battle)                              |
//! | `BlockersDeclaredAnswer`                      | `Block`                                                      |
//! | `CombatDamageAssignedAnswer`                  | `Shares`                                                     |
//! | `CardsDistributedAnswer`                      | `Placements`                                                 |
//! | `TargetsChosenAnswer`                         | `Targets`                                                    |
//! | `MayChosenAnswer`                             | `Yes` / `No` (yes to a mulligan prompt takes the mulligan)   |
//! | `ModesChosenAnswer`                           | `Modes`, or `Subtype` to a creature-type question            |
//! | `ReplacementChosenAnswer`                     | `Index`                                                      |
//! | `TriggersOrderedAnswer`                       | `Order`                                                      |
//! | `CardNamedAnswer`                             | `Name`                                                       |
//! | `PlayerChosenAnswer`                          | `Player`                                                     |
//! | `ColorsChosenAnswer`                          | `Colors`                                                     |
//! | `TargetsDividedAnswer`                        | `Divided`                                                    |
//! | `CountersRemovedAnswer`                       | `Counters`                                                   |
//! | `AnnounceAnswer`                              | `Announce`                                                   |

use cantrip_engine::*;

use crate::handles::HandleSpace;
use crate::protocol::*;
use crate::view::*;

/// Why an answer could not even be put to the engine.
pub enum Untranslatable {
    UnknownHandle(String),
    Illegal(String),
}

fn object(handles: &HandleSpace, game: &Game, h: ViewHandle) -> Result<ObjectId, Untranslatable> {
    match handles.object(h) {
        Some(id) if !is_ability_key(id) && id.index() < game.state.objects.len() => Ok(id),
        _ => Err(Untranslatable::UnknownHandle(format!("handle {} names nothing this seat can see", h.0))),
    }
}

fn target(handles: &HandleSpace, game: &Game, t: &TargetMessage) -> Result<Target, Untranslatable> {
    if t.is_player() {
        return player_at(t.player).map(Target::Player).ok_or_else(|| Untranslatable::Illegal("no such seat".into()));
    }
    match handles.object(t.object) {
        Some(id) if is_ability_key(id) => Ok(Target::Ability(id.0 & 0x7fff_ffff)),
        Some(id) if id.index() < game.state.objects.len() => Ok(Target::Object(id)),
        _ => Err(Untranslatable::UnknownHandle(format!("handle {} names nothing this seat can see", t.object.0))),
    }
}

fn cards(handles: &HandleSpace, game: &Game, list: &[ViewHandle]) -> Result<Vec<ObjectId>, Untranslatable> {
    list.iter().map(|&h| object(handles, game, h)).collect()
}

fn kind_of(kind: ActionKindView) -> Result<ActionKind, Untranslatable> {
    Ok(match kind {
        ActionKindView::PlayLand => ActionKind::PlayLand,
        ActionKindView::CastSpell => ActionKind::CastSpell,
        ActionKindView::ActivateAbility => ActionKind::ActivateAbility,
        ActionKindView::CancelCasting => ActionKind::CancelCasting,
        ActionKindView::PayCost => ActionKind::PayCost,
        ActionKindView::DeclineCost => ActionKind::DeclineCost,
        ActionKindView::AssignMana => ActionKind::AssignMana,
        ActionKindView::ConfirmCasting => ActionKind::ConfirmCasting,
        ActionKindView::IncreaseX => ActionKind::IncreaseX,
        ActionKindView::DecreaseX => ActionKind::DecreaseX,
        ActionKindView::TapToPay => ActionKind::TapToPay,
        ActionKindView::ExileToPay => ActionKind::ExileToPay,
        // Turning a face-down permanent face up is its one ability (CR 708.2a), offered as an activation.
        ActionKindView::TurnFaceUp => ActionKind::ActivateAbility,
        // Unlocking a Room's door is the intrinsic ability it was offered as (CR 709.5e).
        ActionKindView::UnlockDoor => ActionKind::ActivateAbility,
        // Taking a companion is a special action of its own (CR 116.2g).
        ActionKindView::TakeCompanion => ActionKind::TakeCompanion,
        other => return Err(Untranslatable::Illegal(format!("{other:?} is not something this engine offers"))),
    })
}

pub fn translate(
    payload: &AnswerPayload,
    request: &Request,
    handles: &HandleSpace,
    game: &Game,
) -> Result<Answer, Untranslatable> {
    Ok(match payload {
        AnswerPayload::PassAnswer(_) => Answer::Pass,
        AnswerPayload::CancelAnnouncementAnswer(_) => Answer::Cancel,
        AnswerPayload::AnnounceAnswer(a) => Answer::Announce(
            u32::try_from(a.value).map_err(|_| Untranslatable::Illegal(format!("{} cannot be announced (CR 107.1)", a.value)))?,
        ),
        AnswerPayload::CardsChosenAnswer(a) => Answer::Cards(cards(handles, game, &a.cards)?),
        AnswerPayload::TakeActionAnswer(a) => {
            let kind = kind_of(a.kind)?;
            let mut action = Action {
                kind,
                source: if a.source.is_none() { None } else { Some(object(handles, game, a.source)?) },
                index: a.ability_index.clamp(0, 255) as u8,
                granted_by: None,
                paid_by: None,
                targets: a.targets.iter().map(|t| target(handles, game, t)).collect::<Result<_, _>>()?,
                color: a.color.map(color_of),
                mana: a.mana.map(mana_of),
                restriction: restriction_of(a.filter, a.effect, &a.spend_subtypes, a.spend_mana_value),
            };
            if let Some(s) = a.sacrifice {
                action.targets.push(Target::Object(object(handles, game, s)?));
            }
            // A handle names an object and every earlier one it was (`HandleSpace::rebind`): the source is the one the
            // offered option names - Channel's mana ability, offered from the card that granted it, which has moved since.
            if let Some(source) = action.source
                && let RequestKind::PayCosts { options, .. } | RequestKind::Priority { options } = &request.kind
                && !options.iter().any(|o| o.kind == kind && o.index == action.index && o.source == Some(source))
                && let Some(o) = options
                    .iter()
                    .find(|o| o.kind == kind && o.index == action.index && o.source.is_some_and(|s| handles.existing(s) == a.source))
            {
                action.source = o.source;
            }
            // A cancel or a decline names no source, whatever the option it matches carries.
            if matches!(kind, ActionKind::CancelCasting | ActionKind::DeclineCost | ActionKind::PayCost | ActionKind::ConfirmCasting)
                && let RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } | RequestKind::Priority { options } = &request.kind
                && let Some(o) = options.iter().find(|o| o.kind == kind)
            {
                action.source = o.source;
                action.index = o.index;
            }
            // What the option is made by (`granted_by`, `paid_by`) stays the engine's: the answer names the option it takes
            // by its place in the question (`option_id`), or, without one, by kind, source and index where only one option
            // has them.
            if let RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } | RequestKind::Priority { options } =
                &request.kind
            {
                let alike = |o: &&ActionOption| o.kind == action.kind && o.source == action.source && o.index == action.index;
                let taken = if a.option_id != 0 {
                    a.option_id
                        .checked_sub(1)
                        .and_then(|i| usize::try_from(i).ok())
                        .and_then(|i| options.get(i))
                        .filter(alike)
                        .ok_or_else(|| Untranslatable::Illegal("that option is not one this question offers".into()))?
                } else {
                    let mut matching = options.iter().filter(alike);
                    match (matching.next(), matching.next()) {
                        (Some(first), Some(second)) if (first.granted_by, first.paid_by) != (second.granted_by, second.paid_by) => {
                            return Err(Untranslatable::Illegal("several options are alike: say which with its OptionId".into()));
                        }
                        (Some(first), _) => first,
                        (None, _) => return Ok(Answer::Act(action)),
                    }
                };
                action.granted_by = taken.granted_by;
                action.paid_by = taken.paid_by;
            }
            Answer::Act(action)
        }
        AnswerPayload::AttackersDeclaredAnswer(a) => {
            let mut out = Vec::new();
            for attack in &a.attacks {
                let at = if attack.target_permanent.is_none() {
                    Target::Player(player_at(attack.target_player).ok_or_else(|| Untranslatable::Illegal("no such seat".into()))?)
                } else {
                    Target::Object(object(handles, game, attack.target_permanent)?)
                };
                out.push((object(handles, game, attack.attacker)?, at));
            }
            Answer::Attack(out)
        }
        AnswerPayload::BlockersDeclaredAnswer(a) => Answer::Block(
            a.blocks
                .iter()
                .map(|b| Ok((object(handles, game, b.blocker)?, object(handles, game, b.attacker)?)))
                .collect::<Result<_, Untranslatable>>()?,
        ),
        AnswerPayload::CombatDamageAssignedAnswer(a) => Answer::Shares(
            a.shares
                .iter()
                .map(|s| {
                    Ok((
                        object(handles, game, s.source)?,
                        if s.creature.is_none() { None } else { Some(object(handles, game, s.creature)?) },
                        s.amount.clamp(0, i32::from(u16::MAX)) as u16,
                    ))
                })
                .collect::<Result<_, Untranslatable>>()?,
        ),
        AnswerPayload::CardsDistributedAnswer(a) => Answer::Placements {
            placements: a.placements.iter().map(|p| cards(handles, game, p)).collect::<Result<_, _>>()?,
            shuffle: a.shuffle,
        },
        AnswerPayload::TargetsChosenAnswer(a) => {
            let targets = a.targets.iter().map(|t| target(handles, game, t)).collect::<Result<Vec<_>, _>>()?;
            if crate::prompt::pile_selection(&request.kind) {
                Answer::Cards(targets.into_iter().map(|t| match t {
                    Target::Object(id) => Ok(id),
                    _ => Err(Untranslatable::Illegal("a pile contains only the offered cards or permanents".into())),
                }).collect::<Result<_, _>>()?)
            } else {
                Answer::Targets(targets)
            }
        }
        AnswerPayload::MayChosenAnswer(a) => {
            if a.accept {
                Answer::Yes
            } else {
                Answer::No
            }
        }
        AnswerPayload::ModesChosenAnswer(a) => {
            if let RequestKind::ChooseSubtype { choices, .. } = &request.kind {
                let i = a.modes.first().copied().unwrap_or(-1);
                let subtype = usize::try_from(i).ok().and_then(|i| choices.get(i)).copied();
                Answer::Subtype(subtype.ok_or_else(|| Untranslatable::Illegal("not one of the types offered".into()))?)
            } else if let RequestKind::ChooseNumber { least, most, .. } = &request.kind {
                if a.modes.len() != 1 { return Err(Untranslatable::Illegal("choose exactly one number".into())); }
                let index = u32::try_from(a.modes[0]).map_err(|_| Untranslatable::Illegal("not a number offered".into()))?;
                let value = least.saturating_add(index);
                if value > *most { return Err(Untranslatable::Illegal("not a number offered".into())); }
                Answer::Announce(value)
            } else {
                Answer::Modes(a.modes.iter().map(|&m| m.clamp(0, 255) as u8).collect())
            }
        }
        AnswerPayload::ReplacementChosenAnswer(a) => Answer::Index(a.index.clamp(0, 255) as u8),
        AnswerPayload::TriggersOrderedAnswer(a) => Answer::Order(a.order.iter().map(|&m| u32::try_from(m).unwrap_or(u32::MAX)).collect()),
        // An empty name is no name, which no name question allows (CR 201.3): never the lookup of "", which a nameless
        // token matches.
        AnswerPayload::CardNamedAnswer(a) if a.name.is_empty() => {
            return Err(Untranslatable::Illegal("a card name must be chosen".into()));
        }
        AnswerPayload::CardNamedAnswer(a) => {
            Answer::Name(Some(card(&a.name).ok_or_else(|| Untranslatable::Illegal(format!("no card is named \"{}\"", a.name)))?))
        }
        AnswerPayload::PlayerChosenAnswer(a) => {
            Answer::Player(player_at(a.player).ok_or_else(|| Untranslatable::Illegal("no such seat".into()))?)
        }
        AnswerPayload::ColorsChosenAnswer(a) => Answer::Colors(a.colors.iter().map(|&c| color_of(c)).collect()),
        AnswerPayload::TargetsDividedAnswer(a) => Answer::Divided(
            a.shares
                .iter()
                .map(|s| Ok((target(handles, game, &s.target)?, u32::try_from(s.amount).unwrap_or(0))))
                .collect::<Result<_, Untranslatable>>()?,
        ),
        AnswerPayload::CountersRemovedAnswer(a) => Answer::Counters(
            a.removals
                .iter()
                .map(|r| {
                    let kind = CounterKind::ALL
                        .into_iter()
                        .find(|k| k.name() == r.counter)
                        .ok_or_else(|| Untranslatable::Illegal(format!("no counter is called \"{}\"", r.counter)))?;
                    Ok((object(handles, game, r.object)?, kind, r.count.clamp(0, i32::from(u16::MAX)) as u16))
                })
                .collect::<Result<_, Untranslatable>>()?,
        ),
    })
}

fn wire_target(handles: &mut HandleSpace, t: Target) -> TargetMessage {
    match t {
        Target::Player(p) => TargetMessage::player(seat(p)),
        Target::Ability(serial) => TargetMessage::object(handles.handle(ability_key(serial))),
        Target::Object(id) => TargetMessage::object(handles.handle(id)),
        Target::None => TargetMessage::player(0),
    }
}

/// An engine answer as the wire's, in a seat's handles: the inverse of [`translate`], for saying what a scripted seat
/// is meant to answer (`Session::scenario`). Objects the seat has no handle for are given one.
pub fn payload_of(answer: &Answer, request: &Request, handles: &mut HandleSpace, game: &Game) -> AnswerPayload {
    match answer {
        Answer::Pass => AnswerPayload::PassAnswer(PassAnswer {}),
        Answer::Cancel => AnswerPayload::CancelAnnouncementAnswer(CancelAnnouncementAnswer {}),
        Answer::Announce(value) => AnswerPayload::AnnounceAnswer(AnnounceAnswer { value: crate::prompt::wire(*value) }),
        Answer::Cards(ids) if crate::prompt::pile_selection(&request.kind) => AnswerPayload::TargetsChosenAnswer(TargetsChosenAnswer {
            targets: ids.iter().map(|&id| wire_target(handles, Target::Object(id))).collect(),
        }),
        Answer::Cards(ids) => AnswerPayload::CardsChosenAnswer(CardsChosenAnswer { cards: ids.iter().map(|&id| handles.handle(id)).collect() }),
        Answer::Act(action) => {
            let source = action.source.map_or(ViewHandle::NONE, |s| handles.handle(s));
            let mut a = TakeActionAnswer::new(crate::prompt::action_kind_view(action.kind), source);
            a.ability_index = i32::from(action.index);
            // The option it takes, by its place in the question (`ActionOptionMessage::option_id`).
            if let RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } | RequestKind::Priority { options } =
                &request.kind
                && let Some(i) = options.iter().position(|o| {
                    o.kind == action.kind
                        && o.source == action.source
                        && o.index == action.index
                        && o.granted_by == action.granted_by
                        && o.paid_by == action.paid_by
                })
            {
                a.option_id = i as i32 + 1;
            }
            a.color = action.color.map(color_view);
            a.mana = action.mana.map(mana_view);
            if let Some(restriction) = action.restriction {
                (a.filter, a.effect, a.spend_subtypes, a.spend_mana_value) = restriction_view(restriction);
            }
            // A mana ability's sacrifice goes to the wire as `Sacrifice`, not a target (`prompt::option_message`).
            let sacrifices = action.kind == ActionKind::ActivateAbility
                && action.source.is_some_and(|s| {
                    matches!(
                        game.chars(s).abilities().nth(action.index as usize).map(|a| game.cards.ability(a)),
                        Some(AbilityDef::Activated(d)) if d.is_mana_ability() && d.cost.permanents.is_some()
                    )
                });
            if sacrifices && let Some(Target::Object(id)) = action.targets.first() {
                a.sacrifice = Some(handles.handle(*id));
            } else {
                a.targets = action.targets.iter().map(|&t| wire_target(handles, t)).collect();
            }
            AnswerPayload::TakeActionAnswer(a)
        }
        Answer::Targets(ts) => AnswerPayload::TargetsChosenAnswer(TargetsChosenAnswer { targets: ts.iter().map(|&t| wire_target(handles, t)).collect() }),
        Answer::Modes(ms) => AnswerPayload::ModesChosenAnswer(ModesChosenAnswer { modes: ms.iter().map(|&m| i32::from(m)).collect() }),
        Answer::Subtype(st) => {
            let index = match &request.kind {
                RequestKind::ChooseSubtype { choices, .. } => choices.iter().position(|c| c == st).map_or(-1, |i| i as i32),
                _ => -1,
            };
            AnswerPayload::ModesChosenAnswer(ModesChosenAnswer { modes: vec![index] })
        }
        Answer::Colors(cs) => AnswerPayload::ColorsChosenAnswer(ColorsChosenAnswer { colors: cs.iter().map(|&c| color_view(c)).collect() }),
        Answer::Yes => AnswerPayload::MayChosenAnswer(MayChosenAnswer { accept: true }),
        Answer::No => AnswerPayload::MayChosenAnswer(MayChosenAnswer { accept: false }),
        Answer::Attack(attacks) => AnswerPayload::AttackersDeclaredAnswer(AttackersDeclaredAnswer {
            attacks: attacks
                .iter()
                .map(|&(attacker, at)| match at {
                    Target::Player(p) => AttackMessage { attacker: handles.handle(attacker), target_player: seat(p), target_permanent: ViewHandle::NONE },
                    other => {
                        let permanent = match other {
                            Target::Object(id) => handles.handle(id),
                            _ => ViewHandle::NONE,
                        };
                        AttackMessage { attacker: handles.handle(attacker), target_player: 0, target_permanent: permanent }
                    }
                })
                .collect(),
        }),
        Answer::Block(blocks) => AnswerPayload::BlockersDeclaredAnswer(BlockersDeclaredAnswer {
            blocks: blocks.iter().map(|&(b, a)| BlockMessage { blocker: handles.handle(b), attacker: handles.handle(a) }).collect(),
        }),
        Answer::Player(p) => AnswerPayload::PlayerChosenAnswer(PlayerChosenAnswer { player: seat(*p) }),
        Answer::Name(id) => AnswerPayload::CardNamedAnswer(CardNamedAnswer { name: id.map_or(String::new(), |c| game.cards.get(c).name.to_string()) }),
        Answer::Order(order) => AnswerPayload::TriggersOrderedAnswer(TriggersOrderedAnswer { order: order.iter().map(|&o| crate::prompt::wire(o)).collect() }),
        Answer::Index(i) => AnswerPayload::ReplacementChosenAnswer(ReplacementChosenAnswer { index: i32::from(*i) }),
        Answer::Shares(shares) => AnswerPayload::CombatDamageAssignedAnswer(CombatDamageAssignedAnswer {
            shares: shares
                .iter()
                .map(|&(source, creature, amount)| DamageShareMessage {
                    source: handles.handle(source),
                    creature: creature.map_or(ViewHandle::NONE, |c| handles.handle(c)),
                    amount: i32::from(amount),
                })
                .collect(),
        }),
        Answer::Placements { placements, shuffle } => AnswerPayload::CardsDistributedAnswer(CardsDistributedAnswer {
            placements: placements.iter().map(|p| p.iter().map(|&id| handles.handle(id)).collect()).collect(),
            shuffle: *shuffle,
        }),
        Answer::Divided(shares) => AnswerPayload::TargetsDividedAnswer(TargetsDividedAnswer {
            shares: shares.iter().map(|&(t, amount)| TargetShareMessage { target: wire_target(handles, t), amount: i32::try_from(amount).unwrap_or(i32::MAX) }).collect(),
        }),
        Answer::Counters(taken) => AnswerPayload::CountersRemovedAnswer(CountersRemovedAnswer {
            removals: taken
                .iter()
                .map(|&(object, kind, n)| CounterPoolMessage { object: handles.handle(object), counter: kind.name().to_string(), count: i32::from(n) })
                .collect(),
        }),
    }
}

#[cfg(test)]
mod option_id_tests {
    use super::*;

    /// A priority question offering one card twice, by two permissions.
    fn fixture() -> (Game, Request, HandleSpace, ViewHandle) {
        let island = cantrip_engine::card("Island").unwrap();
        let game = Game::new(cantrip_engine::cards(), &Setup { decks: vec![vec![island; 20]; 2], seed: 1, ..Setup::default() });
        let source = ObjectId(0);
        let mut handles = HandleSpace::new();
        let handle = handles.handle(source);
        let first = ActionOption::on(ActionKind::CastSpell, source, 0);
        let second = ActionOption { granted_by: Some(Grant { object: ObjectId(1), ability: AbilityRef::default() }), ..first.clone() };
        let request = Request { id: 1, player: PlayerId(0), may_cancel: false, kind: RequestKind::Priority { options: vec![first, second] } };
        (game, request, handles, handle)
    }

    #[test]
    fn the_option_taken_is_the_one_its_id_names_and_round_trips() {
        let (game, request, mut handles, source) = fixture();
        let mut taken = TakeActionAnswer::new(ActionKindView::CastSpell, source);
        taken.option_id = 2;
        let Ok(answer) = translate(&AnswerPayload::TakeActionAnswer(taken), &request, &handles, &game) else { panic!("an offered option") };
        let Answer::Act(action) = &answer else { panic!("an action") };
        assert_eq!(action.granted_by, Some(Grant { object: ObjectId(1), ability: AbilityRef::default() }));
        let AnswerPayload::TakeActionAnswer(back) = payload_of(&answer, &request, &mut handles, &game) else { panic!("an action") };
        assert_eq!(back.option_id, 2);
    }

    #[test]
    fn an_answer_without_an_id_is_refused_where_two_options_are_alike() {
        let (game, request, handles, source) = fixture();
        let taken = TakeActionAnswer::new(ActionKindView::CastSpell, source);
        assert!(matches!(translate(&AnswerPayload::TakeActionAnswer(taken), &request, &handles, &game), Err(Untranslatable::Illegal(_))));
    }

    #[test]
    fn an_id_the_question_has_not_or_of_another_option_is_refused() {
        let (game, request, handles, source) = fixture();
        for id in [i32::MIN, -1, 3, i32::MAX] {
            let mut taken = TakeActionAnswer::new(ActionKindView::CastSpell, source);
            taken.option_id = id;
            assert!(matches!(translate(&AnswerPayload::TakeActionAnswer(taken), &request, &handles, &game), Err(Untranslatable::Illegal(_))));
        }
        let mut taken = TakeActionAnswer::new(ActionKindView::CastSpell, source);
        taken.option_id = 1;
        taken.ability_index = 1;
        assert!(matches!(translate(&AnswerPayload::TakeActionAnswer(taken), &request, &handles, &game), Err(Untranslatable::Illegal(_))));
    }
}
