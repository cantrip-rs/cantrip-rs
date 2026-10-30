//! A factored answer turned into the engine's, and back (docs/ai-player-plan.md, "Answers"). A policy answers a request
//! step by step through `cantrip_obs::factor` in canonical terms; `Answering` holds what turns that answer into the
//! engine's - the canonical index's engine ids, which `observe_indexed` keeps outside the observation, and the
//! permutations from the ask's sorted lists back to the engine's (`ask::Perms`) - and checks the result with the
//! engine's validator before it is sent. A refused answer is a `Rejection`, for the caller to mask and backtrack.
//!
//! Payment is no policy step: `AutoPay`, and paying an "unless pays" cost, are made by `pay::pay` one engine answer at a
//! time (the request comes back while there is more to pay), as is X, raised or lowered a step at a time to the bucket
//! chosen.

use std::collections::HashMap;

use cantrip_engine::*;
use cantrip_obs::factor::{self, COLORLESS, FactoredAnswer, NO_COLOR, Token};
use cantrip_obs::{Ask, AskKind, ObjectObs, Observation, OptionObs, Ref, action};

use crate::ask::{Perms, ask_of};
use crate::observe::{kind_name, target_ref};
use crate::pay;

/// An answer refused: the request's kind, the engine answer refused (if one was made) and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Rejection {
    pub kind: String,
    pub answer: Option<Answer>,
    pub why: String,
}

impl Rejection {
    /// Whether it was refused for a property of the whole answer the factoring leaves to the validator (see
    /// `cantrip_obs::factor`): menace and the most blockers' requirements (CR 509.1b-c), the most attack requirements
    /// where a cap on attackers leaves a declaration short of them (CR 508.1c-d), a name, a selection rule or a rule of a
    /// target slot's targets together (CR 601.2c), or more wanted than there was to choose. Anything else refused is a
    /// fault of the factoring: a total power or mana value short of its minimum is one, since the factoring offers a card
    /// only while the minimum stays within reach, and cancels a request it can't reach.
    pub fn global(&self) -> bool {
        const GLOBAL: [&str; 10] = [
            "can't be blocked except by",
            "(CR 509.1c)",
            "(CR 508.1d)",
            "not the name of a card",
            "card name",
            "split card's names",
            " chosen, ",
            " found, ",
            "needs a destination",
            " together (CR 601.2c)",
        ];
        GLOBAL.iter().any(|g| self.why.contains(g))
    }
}

/// What turns a factored answer to one request into the engine's.
#[derive(Clone, Debug)]
pub struct Answering {
    pub ask: Ask,
    perms: Perms,
    index: Vec<ObjectId>,
    refs: HashMap<ObjectId, u32>,
}

/// An option's colors as the factoring offers them: the color step's existence (`cantrip_obs::factor`).
fn has_color_step(o: &OptionObs) -> bool {
    o.color_choices != 0 || (o.kind == action::ACTIVATE_ABILITY && o.mana == Some(COLORLESS as u8))
}

impl Answering {
    /// The engine's priority option that the factoring's option `n` is (a `Tok::Option` token's number), where `request`
    /// is a priority.
    pub fn priority_option<'a>(&self, request: &'a Request, n: u16) -> Option<&'a ActionOption> {
        match &request.kind {
            RequestKind::Priority { options } => self.perms.options.get(n as usize).and_then(|&i| options.get(i)),
            _ => None,
        }
    }

    /// For `request`, asked of the seat `observation` is of, with `index` the engine id of each canonical index (as
    /// `observe_indexed` gives them).
    pub fn new(
        game: &Game,
        request: &Request,
        observation: &Observation,
        index: &[ObjectId],
    ) -> Answering {
        let refs: HashMap<ObjectId, u32> = index
            .iter()
            .enumerate()
            .filter(|&(i, _)| {
                !matches!(
                    observation.objects.get(i),
                    Some(ObjectObs::Hidden { .. }) | None
                )
            })
            .map(|(i, &id)| (id, i as u32))
            .collect();
        let (ask, perms) = ask_of(game, request, &refs);
        Answering {
            ask,
            perms,
            index: index.to_vec(),
            refs,
        }
    }

    fn id(&self, r: Ref) -> Result<ObjectId, String> {
        match r {
            Ref::Object(i) => self
                .index
                .get(i as usize)
                .copied()
                .ok_or_else(|| format!("no object {i}")),
            other => Err(format!("{other:?} is no object")),
        }
    }

    fn target(&self, r: Ref) -> Result<Target, String> {
        Ok(match r {
            Ref::None => Target::None,
            Ref::Player(p) => Target::Player(PlayerId(p)),
            Ref::Object(_) => {
                let id = self.id(r)?;
                if id.0 & 0x8000_0000 != 0 {
                    Target::Ability(id.0 & 0x7fff_ffff)
                } else {
                    Target::Object(id)
                }
            }
        })
    }

    fn r(&self, id: ObjectId) -> Ref {
        target_ref(Target::Object(id), &self.refs)
    }

    fn t(&self, t: Target) -> Ref {
        target_ref(t, &self.refs)
    }

    /// The engine's answer to `request` that `answer` makes, checked by the engine's validator.
    pub fn engine(
        &self,
        game: &Game,
        request: &Request,
        answer: &FactoredAnswer,
    ) -> Result<Answer, Rejection> {
        let reject = |answer: Option<Answer>, why: String| Rejection {
            kind: kind_name(&request.kind),
            answer,
            why,
        };
        let made = self
            .make(game, request, answer)
            .map_err(|why| reject(None, why))?;
        check_answer(game, request, &made).map_err(|r| reject(Some(made.clone()), r.0))?;
        Ok(made)
    }

    fn make(
        &self,
        game: &Game,
        request: &Request,
        answer: &FactoredAnswer,
    ) -> Result<Answer, String> {
        use FactoredAnswer as F;
        let options: &[ActionOption] = match &request.kind {
            RequestKind::Priority { options }
            | RequestKind::PayCosts { options, .. }
            | RequestKind::UnlessPays { options, .. } => options,
            _ => &[],
        };
        let offered = |kind: ActionKind| {
            options
                .iter()
                .find(|o| o.kind == kind)
                .map(|o| Answer::Act(Action::of(o)))
        };
        let cancel = || offered(ActionKind::CancelCasting).unwrap_or(Answer::Cancel);
        let ids = |refs: &[Ref]| {
            refs.iter()
                .map(|&r| self.id(r))
                .collect::<Result<Vec<_>, _>>()
        };
        Ok(match (&request.kind, answer) {
            (_, F::Cancel) => cancel(),
            (RequestKind::Priority { .. }, F::Pass) => Answer::Pass,
            (
                RequestKind::Priority { options },
                F::Act {
                    option,
                    targets,
                    color,
                },
            ) => {
                let o = &options[*self
                    .perms
                    .options
                    .get(*option as usize)
                    .ok_or("no such option")?];
                let mut a = Action::of(o);
                if o.kind != ActionKind::AssignMana {
                    a.mana = None;
                }
                a.targets = targets
                    .iter()
                    .map(|&t| self.target(t))
                    .collect::<Result<_, _>>()?;
                match color.unwrap_or(NO_COLOR) {
                    c if c < 5 => a.color = Some(Color::ALL[c as usize]),
                    COLORLESS => a.mana = Some(Mana::C),
                    _ => {}
                }
                Answer::Act(a)
            }
            (
                RequestKind::PayCosts {
                    announcing: true,
                    x,
                    ..
                },
                F::X(want),
            ) => {
                let step = if *x < u32::from(*want) {
                    offered(ActionKind::IncreaseX)
                } else if *x > u32::from(*want) {
                    offered(ActionKind::DecreaseX)
                } else {
                    None
                };
                step.or_else(|| offered(ActionKind::ConfirmCasting))
                    .ok_or("nothing announces X")?
            }
            (
                RequestKind::PayCosts {
                    announcing: false, ..
                },
                F::AutoPay,
            ) => match pay::pay(game, request, None) {
                Some(a) => a,
                None if self.ask.may_cancel || offered(ActionKind::CancelCasting).is_some() => {
                    cancel()
                }
                None => return Err("nothing pays the cost".into()),
            },
            (RequestKind::UnlessPays { .. }, F::Unless { pay: false, .. }) => {
                offered(ActionKind::DeclineCost).ok_or("the cost cannot be declined")?
            }
            (RequestKind::UnlessPays { .. }, F::Unless { pay: true, x }) => {
                match pay::pay(game, request, *x) {
                    Some(a) => a,
                    None => offered(ActionKind::DeclineCost)
                        .ok_or("nothing pays the cost, and it cannot be declined")?,
                }
            }
            (RequestKind::ChooseTarget { .. }, F::Refs(refs)) => Answer::Targets(
                refs.iter()
                    .map(|&r| self.target(r))
                    .collect::<Result<_, _>>()?,
            ),
            (_, F::Refs(refs)) => Answer::Cards(ids(refs)?),
            (RequestKind::ChooseNumber { least, most, .. }, F::Modes(m)) => {
                if m.len() != 1 { return Err("choose exactly one number".into()); }
                let value = least.checked_add(u32::from(m[0])).ok_or("number is outside the offered range")?;
                if value > *most { return Err("number is outside the offered range".into()); }
                Answer::Announce(value)
            }
            (_, F::Modes(m)) => Answer::Modes(m.clone()),
            (_, F::Colors(c)) => {
                Answer::Colors(c.iter().map(|&c| Color::ALL[c as usize]).collect())
            }
            (_, F::Yes) => Answer::Yes,
            (_, F::No) => Answer::No,
            (_, F::Player(p)) => Answer::Player(PlayerId(*p)),
            // No name of the network's own is the question's default (CR 201.3: a name must be chosen): its suggestion, or
            // the first name it allows.
            (RequestKind::ChooseName { suggestion, kind, .. }, F::Name(None)) => Answer::Name(Some(
                default_name(game.cards, *kind, *suggestion).ok_or("no card's name is of the kind asked for")?,
            )),
            (_, F::Name(Some(n))) => Answer::Name(Some(
                game.cards
                    .find(n)
                    .ok_or_else(|| format!("no card is named {n}"))?,
            )),
            (_, F::Subtype(s)) => {
                Answer::Subtype(*Subtype::ALL.get(*s as usize).ok_or("no such subtype")?)
            }
            (_, F::Attack(attacks)) => Answer::Attack(
                attacks
                    .iter()
                    .map(|&(c, d)| Ok((self.id(c)?, self.target(d)?)))
                    .collect::<Result<_, String>>()?,
            ),
            (_, F::Block(blocks)) => Answer::Block(
                blocks
                    .iter()
                    .map(|&(b, a)| Ok((self.id(b)?, self.id(a)?)))
                    .collect::<Result<_, String>>()?,
            ),
            (_, F::Shares(shares)) => Answer::Shares(
                shares
                    .iter()
                    .map(|&(s, c, n)| Ok((self.id(s)?, c.map(|c| self.id(c)).transpose()?, n)))
                    .collect::<Result<_, String>>()?,
            ),
            (
                RequestKind::DistributeCards { destinations, .. },
                F::Placements { shuffle: true, .. },
            ) => Answer::Placements {
                placements: vec![Vec::new(); destinations.len()],
                shuffle: true,
            },
            (
                _,
                F::Placements {
                    placements,
                    shuffle,
                },
            ) => Answer::Placements {
                placements: placements
                    .iter()
                    .map(|p| ids(p))
                    .collect::<Result<_, _>>()?,
                shuffle: *shuffle,
            },
            (_, F::Divided(shares)) => Answer::Divided(
                shares
                    .iter()
                    .map(|&(t, n)| Ok((self.target(t)?, u32::from(n))))
                    .collect::<Result<_, String>>()?,
            ),
            (_, F::Counters(taken)) => Answer::Counters(
                taken
                    .iter()
                    .map(|&(o, k, n)| {
                        Ok((
                            self.id(o)?,
                            *CounterKind::ALL.get(k as usize).ok_or("no such counter")?,
                            n,
                        ))
                    })
                    .collect::<Result<_, String>>()?,
            ),
            (_, F::Order(order)) => Answer::Order(
                order
                    .iter()
                    .map(|&i| {
                        self.perms
                            .order
                            .get(i as usize)
                            .map(|&e| e as u32)
                            .ok_or("no such trigger")
                    })
                    .collect::<Result<_, _>>()?,
            ),
            (_, F::Index(i)) => Answer::Index(
                self.perms
                    .order
                    .get(*i as usize)
                    .map(|&e| e as u8)
                    .ok_or("no such effect")?,
            ),
            (kind, answer) => {
                return Err(format!("{answer:?} is no answer to {}", kind_name(kind)));
            }
        })
    }

    /// The factored answer an engine answer is, in canonical terms; `x` the X it goes on to announce or pay at (see
    /// `final_x`). `None` where the answer names what the seat cannot see, or is no answer the factoring makes.
    pub fn factored(
        &self,
        request: &Request,
        answer: &Answer,
        x: Option<u16>,
    ) -> Option<FactoredAnswer> {
        use FactoredAnswer as F;
        let all = |refs: Vec<Ref>| (!refs.contains(&Ref::None)).then_some(refs);
        let is = |kind: ActionKind| matches!(answer, Answer::Act(a) if a.kind == kind);
        if *answer == Answer::Cancel || is(ActionKind::CancelCasting) {
            return Some(F::Cancel);
        }
        Some(match (&request.kind, answer, &self.ask.kind) {
            (RequestKind::Priority { .. }, Answer::Pass, _) => F::Pass,
            (
                RequestKind::Priority { options },
                Answer::Act(a),
                AskKind::Priority { options: canonical },
            ) => {
                let engine = options.iter().position(|o| {
                    o.kind == a.kind
                        && o.source == a.source
                        && o.index == a.index
                        && (o.granted_by, o.paid_by) == (a.granted_by, a.paid_by)
                        && (o.kind != ActionKind::AssignMana
                            || (o.mana, o.restriction) == (a.mana, a.restriction))
                })?;
                let option = self.perms.options.iter().position(|&e| e == engine)?;
                let o = &canonical[option];
                let color = has_color_step(o).then(|| match (a.color, a.mana) {
                    (Some(c), _) => c as u16,
                    (None, Some(Mana::C)) if a.kind == ActionKind::ActivateAbility => COLORLESS,
                    _ => NO_COLOR,
                });
                F::Act {
                    option: option as u16,
                    targets: all(a.targets.iter().map(|&t| self.t(t)).collect())?,
                    color,
                }
            }
            (
                RequestKind::PayCosts {
                    announcing: true,
                    x: now,
                    ..
                },
                _,
                _,
            ) => F::X(x.unwrap_or(u16::try_from(*now).unwrap_or(u16::MAX))),
            (RequestKind::PayCosts { .. }, _, _) => F::AutoPay,
            (RequestKind::UnlessPays { .. }, Answer::Act(a), _)
                if a.kind == ActionKind::DeclineCost =>
            {
                F::Unless {
                    pay: false,
                    x: None,
                }
            }
            (RequestKind::UnlessPays { x: now, .. }, Answer::Act(_), _) => F::Unless {
                pay: true,
                x: now.map(|n| x.unwrap_or(n)),
            },
            (_, Answer::Cards(ids), _) => F::Refs(all(ids.iter().map(|&id| self.r(id)).collect())?),
            (_, Answer::Targets(ts), _) => F::Refs(all(ts.iter().map(|&t| self.t(t)).collect())?),
            (RequestKind::ChooseNumber { least, most, .. }, Answer::Announce(n), _) if (*least..=*most).contains(n) => {
                F::Modes(vec![u8::try_from(n - least).expect("ChooseNumber range is bounded by the mechanic")])
            }
            (_, Answer::Modes(m), _) => F::Modes(m.clone()),
            (_, Answer::Colors(c), _) => F::Colors(c.iter().map(|&c| c as u8).collect()),
            (_, Answer::Yes, _) => F::Yes,
            (_, Answer::No, _) => F::No,
            (_, Answer::Player(p), _) => F::Player(p.0),
            (_, Answer::Subtype(s), _) => F::Subtype(*s as u16),
            (_, Answer::Attack(attacks), _) => F::Attack(
                attacks
                    .iter()
                    .map(|&(c, d)| {
                        Some((self.r(c), self.t(d)))
                            .filter(|(c, d)| *c != Ref::None && *d != Ref::None)
                    })
                    .collect::<Option<_>>()?,
            ),
            (_, Answer::Block(blocks), _) => F::Block(
                blocks
                    .iter()
                    .map(|&(b, a)| {
                        Some((self.r(b), self.r(a)))
                            .filter(|(b, a)| *b != Ref::None && *a != Ref::None)
                    })
                    .collect::<Option<_>>()?,
            ),
            (_, Answer::Shares(shares), _) => F::Shares(
                shares
                    .iter()
                    .map(|&(s, c, n)| {
                        let c = c.map(|c| self.r(c));
                        (self.r(s) != Ref::None && c != Some(Ref::None)).then(|| (self.r(s), c, n))
                    })
                    .collect::<Option<_>>()?,
            ),
            (
                _,
                Answer::Placements { shuffle: true, .. },
                AskKind::DistributeCards { destinations, .. },
            ) => F::Placements {
                placements: vec![Vec::new(); destinations.len()],
                shuffle: true,
            },
            (_, Answer::Placements { placements, .. }, _) => F::Placements {
                placements: placements
                    .iter()
                    .map(|p| all(p.iter().map(|&id| self.r(id)).collect()))
                    .collect::<Option<_>>()?,
                shuffle: false,
            },
            (_, Answer::Divided(shares), _) => F::Divided(
                shares
                    .iter()
                    .map(|&(t, n)| {
                        Some((self.t(t), u16::try_from(n).unwrap_or(u16::MAX)))
                            .filter(|s| s.0 != Ref::None)
                    })
                    .collect::<Option<_>>()?,
            ),
            (_, Answer::Counters(taken), _) => F::Counters(
                taken
                    .iter()
                    .map(|&(o, k, n)| Some((self.r(o), k as u16, n)).filter(|t| t.0 != Ref::None))
                    .collect::<Option<_>>()?,
            ),
            (_, Answer::Order(order), _) => F::Order(
                order
                    .iter()
                    .map(|&e| {
                        self.perms
                            .order
                            .iter()
                            .position(|&p| p == e as usize)
                            .map(|i| i as u8)
                    })
                    .collect::<Option<_>>()?,
            ),
            (_, Answer::Index(e), _) => {
                F::Index(self.perms.order.iter().position(|&p| p == *e as usize)? as u8)
            }
            _ => return None,
        })
    }

    /// The tokens that make `answer` (`cantrip_obs::factor::decompose`), with `names` the names a name is chosen among.
    pub fn decompose(
        &self,
        names: &[String],
        answer: &FactoredAnswer,
    ) -> Result<Vec<Token>, String> {
        factor::decompose(&self.ask, names, answer)
    }
}

/// The factored answer a name answer is, given the card names: `Answer::Name` carries a card id.
pub fn factored_name(game: &Game, answer: &Answer) -> Option<FactoredAnswer> {
    match answer {
        Answer::Name(n) => Some(FactoredAnswer::Name(
            n.map(|c| game.cards.get(c).name.to_string()),
        )),
        _ => None,
    }
}

/// The X an announcement (or an "unless pays" cost's X) comes to: `now`, moved a step by each of the leading `answers`
/// that raises or lowers it.
pub fn final_x(now: u32, answers: &[Answer]) -> u32 {
    let mut x = now;
    for a in answers {
        match a {
            Answer::Announce(n) => x = *n,
            Answer::Act(a) if a.kind == ActionKind::IncreaseX => x += 1,
            Answer::Act(a) if a.kind == ActionKind::DecreaseX => x = x.saturating_sub(1),
            _ => break,
        }
    }
    x
}
