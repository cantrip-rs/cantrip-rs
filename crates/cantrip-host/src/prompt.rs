//! The engine's requests as the wire's prompts, in a seat's own handle space.
//!
//! The native requests mirror the C# prompts kind for kind, so most of this is a field-for-field transcription. The
//! mapping, request by request (docs/native-host.md in the report has the same table):
//!
//! | native `RequestKind`   | wire prompt                                                                        |
//! |------------------------|------------------------------------------------------------------------------------|
//! | `Priority`             | `PriorityPromptMessage`                                                            |
//! | `PayCosts`             | `PayCostsPromptMessage` (announcing X/kicks sets `AnnouncesXOrKicks`)              |
//! | `UnlessPays`           | `UnlessPaysPromptMessage`                                                          |
//! | `ChooseCards`          | `ChooseCardsPromptMessage`                                                         |
//! | `TapForPower`          | `ChooseCardsPromptMessage` (purpose `TapToPay`, `Powers`, `MinimumPower`; harmonize's: `TapToReduce`, up to one) |
//! | `Craft`                | `ChooseCardsPromptMessage` (purpose `Craft`, the quality as `Kind`, `Minimum` for "one or more")               |
//! | `ChooseTarget`         | `ChooseTargetPromptMessage`                                                        |
//! | `ChooseModes`          | `ChooseModesPromptMessage`                                                         |
//! | `ChooseSubtype`        | `ChooseModesPromptMessage` over the subtypes (the C# engine asks it that way too)  |
//! | `ChooseColors`         | `ChooseColorsPromptMessage`                                                        |
//! | `ChooseName`           | `NameCardPromptMessage`                                                            |
//! | `May`                  | `MayPromptMessage` (the verb is the purpose's name)                                |
//! | `ChoosePlayer`         | `ChoosePlayerPromptMessage`                                                        |
//! | `ChooseDividedTargets` | `ChooseDividedTargetsPromptMessage`                                                |
//! | `RemoveCounters`       | `RemoveCountersPromptMessage`                                                      |
//! | `DeclareAttackers`     | `DeclareAttackersPromptMessage`                                                    |
//! | `AssignCombatDamage`   | `AssignCombatDamagePromptMessage`                                                  |
//! | `DeclareBlockers`      | `DeclareBlockersPromptMessage`                                                     |
//! | `DiscardToHandSize`    | `DiscardPromptMessage`                                                             |
//! | `Mulligan`             | `MulliganPromptMessage`                                                            |
//! | `ChooseStartingPlayer` | `ChooseStartingPlayerPromptMessage`                                                |
//! | `OrderTriggers`        | `OrderTriggersPromptMessage`                                                       |
//! | `DistributeCards`      | `DistributeCardsPromptMessage`                                                     |
//! | `SearchLibrary`        | `SearchLibraryPromptMessage`                                                       |
//! | `ChooseReplacement`    | `ChooseReplacementPromptMessage`                                                   |

use cantrip_engine::*;

use crate::handles::HandleSpace;
use crate::protocol::*;
use crate::view::*;

/// What a prompt may not say about an object that is gone: a source that has left and is no longer in the view is no
/// handle, and the client draws the ability from its text.
fn source_handle(game: &Game, handles: &mut HandleSpace, id: ObjectId) -> ViewHandle {
    let existing = handles.existing(id);
    if !existing.is_none() {
        existing
    } else if id.index() < game.state.objects.len() && live(&game.state, id) {
        handles.handle(id)
    } else {
        ViewHandle::NONE
    }
}

/// A replacement effect's source by name, for a seat that may hold no handle to it: one that has left - its last-known
/// name (CR 608.2h) - and was seen where it was. Nothing for an object in a zone the seat may not look into (a hand, a
/// library); a face-down one has no name (CR 708.2).
fn source_name(game: &Game, viewer: PlayerId, id: ObjectId) -> String {
    if id.index() >= game.state.objects.len() {
        return String::new();
    }
    let zone = game.state.obj(id).zone;
    if zone.is(ZoneKind::Gone) || crate::view::looks_into(viewer, zone) { game.name(id).to_string() } else { String::new() }
}

fn target_message(handles: &mut HandleSpace, target: Target) -> Option<TargetMessage> {
    match target {
        Target::Player(p) => Some(TargetMessage::player(seat(p))),
        Target::Object(id) => Some(TargetMessage::object(handles.handle(id))),
        Target::Ability(serial) => Some(TargetMessage::object(handles.handle(ability_key(serial)))),
        Target::None => None,
    }
}

pub(crate) fn action_kind_view(kind: ActionKind) -> ActionKindView {
    match kind {
        ActionKind::PlayLand => ActionKindView::PlayLand,
        ActionKind::CastSpell => ActionKindView::CastSpell,
        ActionKind::ActivateAbility => ActionKindView::ActivateAbility,
        ActionKind::CancelCasting => ActionKindView::CancelCasting,
        ActionKind::PayCost => ActionKindView::PayCost,
        ActionKind::DeclineCost => ActionKindView::DeclineCost,
        ActionKind::AssignMana => ActionKindView::AssignMana,
        ActionKind::ConfirmCasting => ActionKindView::ConfirmCasting,
        ActionKind::IncreaseX => ActionKindView::IncreaseX,
        ActionKind::DecreaseX => ActionKindView::DecreaseX,
        ActionKind::TapToPay => ActionKindView::TapToPay,
        ActionKind::ExileToPay => ActionKindView::ExileToPay,
    }
}

/// A cost as rules text, for a mana ability's row in a menu.
fn cost_text_of(cost: &Cost, name: &str) -> String {
    let mut parts = Vec::new();
    if cost.mana != ManaCost::ZERO {
        parts.push(cost.mana.to_string());
    }
    if cost.tap {
        parts.push("{T}".to_string());
    }
    if cost.energy > 0 {
        parts.push(format!("Pay {}", "{E}".repeat(cost.energy as usize)));
    }
    if cost.life > 0 {
        parts.push(format!("Pay {} life", cost.life));
    }
    if cost.sacrifice_self {
        parts.push(format!("Sacrifice {name}"));
    }
    if let Some(p) = cost.permanents {
        let verb = match p.action {
            _ if cost.tap_permanents => "Tap",
            PermanentAction::Sacrifice => "Sacrifice",
            PermanentAction::ReturnToHand => "Return",
            PermanentAction::Exile => "Exile",
        };
        let what = if p.count == 1 { "a permanent".to_string() } else { format!("{} permanents", p.count) };
        parts.push(format!("{verb} {what}"));
    }
    if cost.discard_self {
        parts.push("Discard".to_string());
    }
    if cost.exile_from_graveyard > 0 {
        let n = cost.exile_from_graveyard;
        parts.push(format!("Exile {n} other card{} from your graveyard", if n == 1 { "" } else { "s" }));
    }
    parts.join(", ")
}

fn option_message(game: &Game, handles: &mut HandleSpace, o: &ActionOption) -> ActionOptionMessage {
    let source = o.source.map(|s| handles.handle(s)).unwrap_or(ViewHandle::NONE);
    let mut m = ActionOptionMessage::new(action_kind_view(o.kind), source);
    m.ability_index = i32::from(o.index);
    // A face-down permanent's own ability that turns it face up (CR 701.40b, 701.58b).
    if o.kind == ActionKind::ActivateAbility
        && let Some(s) = o.source
        && game.chars(s).abilities().nth(o.index as usize).is_some_and(|a| game.cards.turns_face_up(a))
    {
        m.kind = ActionKindView::TurnFaceUp;
    }
    // A Room's locked door is unlocked by paying its mana cost (CR 709.5e): its own kind, saying which door and what it costs.
    if o.kind == ActionKind::ActivateAbility
        && let Some(s) = o.source
        && let Some(a) = game.chars(s).abilities().nth(o.index as usize)
        && a.card == game.cards.intrinsic
        && matches!(a.index, intrinsic::UNLOCK_LEFT | intrinsic::UNLOCK_RIGHT)
    {
        let room = game.cards.get(game.chars(s).card);
        if let Some((left, right)) = room.split {
            let door = if a.index == intrinsic::UNLOCK_LEFT { left } else { right };
            m.kind = ActionKindView::UnlockDoor;
            m.cost_text = door.cost.to_string();
            m.spell_name = door.name.to_string();
        }
    }
    m.targets_required = wire(o.targets_required);
    m.targets_optional = o.targets_optional;
    m.color_choices = colors_view(o.color_choices);
    m.description = describe(game.cards, &game.state, &game.log, o).to_string();
    if m.kind == ActionKindView::UnlockDoor {
        m.description = format!("Unlock {} {}", m.spell_name, m.cost_text);
    }
    if let Some(mana) = o.mana {
        m.produces = vec![mana_view(mana)];
    }
    // What restricts the mana: of pool mana to assign, or of the mana a mana ability adds (CR 106.6).
    if let Some(restriction) = o.restriction {
        (m.filter, m.effect, m.spend_subtypes, m.spend_mana_value) = restriction_view(restriction);
    }
    m.pays_nothing = o.pays_nothing;
    let candidates: Vec<TargetMessage> = o.target_candidates.iter().filter_map(|t| target_message(handles, *t)).collect();

    let Some(src) = o.source else {
        m.target_candidates = candidates;
        return m;
    };
    let def = game.cards.get(game.state.obj(src).card);
    match o.kind {
        ActionKind::CastSpell => {
            // Cast as its Adventure or its Omen (CR 715.3, 720.3), it is that spell: its name, and its own mana cost.
            let inset = def.inset_face(o.index);
            // A cast another object makes possible names the face it casts (a Room's door, an Adventure): its name and its cost.
            // So does a cast for an alternative cost another object gives (CR 118.9: Dracogenesis), of a face it casts.
            let granted_face = (o.granted_by.is_some() || o.paid_by.is_some())
                .then(|| cast_face_of(game, src, o.index))
                .filter(|&face| !std::ptr::eq(face, def));
            if let Some(face) = granted_face.or(inset) {
                m.spell_name = face.name.to_string();
            }
            if let Some(face) = inset {
                m.cost_text = face.cost.to_string();
            }
            // A cast a permanent grants (CR 604, 118.9) says what it costs, which the way's text need not: Iroh's flashback
            // "equal to that card's mana cost", Underworld Breach's escape. A face it casts shows it as its cost; the card
            // itself, beside the way's text, unless that text already names a cost ("Flashback {1}").
            // Whoever is asked casts it: a card in an opponent's exile cast under a permission is not its owner's cast.
            let asked = game.pending().map_or(game.state.turn.priority, |r| r.player);
            if let Some(cost) = granted_cast_cost(game, asked, o) {
                let text = cost_text_of(&cost, def.name);
                if granted_face.is_some() {
                    m.cost_text = text;
                } else if !text.is_empty() && !m.description.contains('{') {
                    m.description = if m.description.is_empty() { text } else { format!("{} {text}", m.description) };
                }
            }
            let player = asked;
            // A way that casts without paying the mana cost (CR 118.9), whose X is 0 (CR 107.3b), is marked so,
            // and its row says what it comes to with the cost changes in play before any target ({0}, or {1} under Thalia).
            let granted = granted_cast_total(game, player, o);
            // An alternative cost a static ability gives (CR 118.9): "without paying its mana cost" (Omniscience, Dracogenesis) is
            // marked so; one in mana ({W}{U}{B}{R}{G} rather than the mana cost) is its cost. Either says what it comes to.
            let alternative = alternative_cast_total(game, player, o);
            let cost = if let Some((paid, total)) = alternative {
                // Rooftop Storm's {0} is nothing paid as well; a face whose own cost is {0} is cast the same either way. Energy
                // paid instead (Primal Prayers' {E}) is a cost, so no cast for nothing: the row's words say it.
                let face = granted_face.unwrap_or(def);
                m.without_paying_mana_cost =
                    paid.energy == 0 && paid.mana == ManaCost::ZERO && (face.cost != ManaCost::ZERO || !face.has_cost);
                m.cost_text = total.to_string();
                paid.mana
            } else if let Some((way, total)) = granted {
                let face = granted_face.unwrap_or(def);
                m.without_paying_mana_cost = way.mana == ManaCost::ZERO && (face.cost != ManaCost::ZERO || !face.has_cost);
                if m.without_paying_mana_cost {
                    m.cost_text = total.to_string();
                }
                way.mana
            } else if let Some(face) = granted_face.or(inset) {
                face.cost
            } else if o.index == 0 {
                def.cost
            } else {
                def.ways.get(o.index as usize - 1).map_or(def.cost, |w| w.cost.mana)
            };
            m.has_x = cost.x > 0;
            // A way whose cost returns tapped permanents, chosen once the mana is paid (web-slinging, CR 702.188a, 601.2g-h).
            let way = match granted {
                Some((way, _)) => Some(way),
                None if granted_face.or(inset).is_none() && o.index > 0 => def.ways.get(o.index as usize - 1).map(|w| w.cost),
                None => None,
            };
            m.tapped_after_mana = way.is_some_and(|c| c.permanents.is_some_and(|p| p.tapped));
            // Paid in life rather than its mana cost (CR 118.9, 119.4: Inside Information, Bolas's Citadel): the life is the
            // cost the row shows and the seats weigh, and no mana cost, which is not paid; an X in it is 0 (CR 107.3).
            if let Some(life) = life_for_cast(game, game.cards, asked, o) {
                m.life_cost = i32::from(life);
                m.cost_text = format!("for {life} life");
                m.has_x = false;
            }
            // A cost that depends on the target it is cast with (CR 601.2c, 601.2f): the row says what the cast costs with
            // each candidate, where that is not what it costs otherwise - mana, and life a change adds (CR 118.8).
            let mut by_target = false;
            if (o.index == 0 || granted.is_some() || alternative.is_some()) && o.targets_required == 1 && !o.target_candidates.is_empty() {
                let costs = cast_costs(game, player, o, &o.target_candidates);
                if costs.iter().any(|&(c, life)| c != cost || life > 0) {
                    let each = costs_by_target(game, &o.target_candidates, &costs);
                    m.description = if m.description.is_empty() { format!("costs {each}") } else { format!("{} (costs {each})", m.description) };
                    by_target = true;
                }
            }
            // A total the cost changes in play make other than the cost it is cast for (CR 601.2f: Thalia's tax, Helm of
            // Awakening's discount, Trinisphere's three): the row says it, so the cast says what it costs before it is paid.
            if !by_target
                && m.life_cost == 0
                && let Some((base, total, life)) = cast_total(game, player, o)
                && (total != base || life > 0)
            {
                let total = if life > 0 { format!("{total} and {life} life") } else { total.to_string() };
                m.description = if m.description.is_empty() { format!("costs {total}") } else { format!("{} (costs {total})", m.description) };
            }
            m.target_candidates = candidates;
        }
        ActionKind::ActivateAbility => {
            let ability = game.chars(src).abilities().nth(o.index as usize).map(|a| game.cards.ability(a));
            if let Some(AbilityDef::Activated(d)) = ability {
                m.life_cost = i32::from(d.cost.life);
                // A cost that returns tapped permanents, chosen once the mana is paid (CR 602.2b, 601.2g-h: Urban Retreat).
                m.tapped_after_mana = !d.is_mana_ability() && d.cost.permanents.is_some_and(|p| p.tapped);
                // A special action whose cost a static ability changes (CR 116.2k, 118.7: "plotting cards from your hand costs
                // {2} less"): the row says what it costs now, which its printed text does not.
                if d.special_action
                    && let Some(cost) = special_action_costs(game, game.state.obj(src).controller, src, o.index)
                {
                    m.description = format!("{} (costs {cost})", m.description);
                }
                if d.is_mana_ability() {
                    for mana in Mana::ALL {
                        for _ in 0..d.adds.get(mana) {
                            m.produces.push(mana_view(mana));
                        }
                    }
                    if !o.color_choices.is_empty() {
                        m.produces_of_any_one_color = i32::from(d.adds_any.max(1));
                    }
                    m.cost_text = cost_text_of(&d.cost, def.name);
                    // A mana ability's sacrifice comes with the activation: the engine takes it as a target.
                    if d.cost.permanents.is_some() && o.targets_required > 0 {
                        m.sacrifice_candidates = candidates.iter().map(|t| t.object).collect();
                        m.targets_required = 0;
                        return m;
                    }
                } else if o.targets_required == 1 && !o.target_candidates.is_empty() {
                    // A cost that depends on the target it is announced with (CR 601.2c, 601.2f, 602.2b): the row says what
                    // the activation costs with each candidate, where that is not the printed cost.
                    let player = game.state.obj(src).controller;
                    let costs = activation_costs(game, player, src, o.index, &o.target_candidates);
                    if costs.iter().any(|&(c, life)| c != d.cost.mana || life > 0) {
                        m.description = format!("{} (costs {})", m.description, costs_by_target(game, &o.target_candidates, &costs));
                    }
                }
            }
            m.target_candidates = candidates;
        }
        ActionKind::TapToPay | ActionKind::ExileToPay => {
            m.alternative = i32::from(o.index);
            m.pays_up_to = if o.index == PAY_CONVOKE { 0 } else { i32::MAX.min(255) };
            m.target_candidates = candidates;
        }
        _ => m.target_candidates = candidates,
    }
    m
}

/// What an action costs with each candidate target, for its menu row: "{1} targeting Dwarven Mauler, {4} targeting
/// Ornithopter".
fn costs_by_target(game: &Game, candidates: &[Target], costs: &[(ManaCost, u32)]) -> String {
    let each: Vec<String> = candidates
        .iter()
        .zip(costs)
        .map(|(t, &(cost, life))| {
            let cost = if life > 0 { format!("{cost} and {life} life") } else { cost.to_string() };
            match t.object() {
                Some(id) => format!("{cost} targeting {}", game.name(id)),
                None => format!("{cost} targeting a player"),
            }
        })
        .collect();
    each.join(", ")
}

fn options_messages(game: &Game, handles: &mut HandleSpace, options: &[ActionOption]) -> Vec<ActionOptionMessage> {
    options
        .iter()
        .enumerate()
        .map(|(i, o)| ActionOptionMessage { option_id: i as i32 + 1, ..option_message(game, handles, o) })
        .collect()
}

fn remaining_colored(cost: &ManaCost) -> Vec<ColorView> {
    let mut out = Vec::new();
    for color in Color::ALL {
        for _ in 0..cost.symbols[color.mana() as usize] {
            out.push(color_view(color));
        }
    }
    out
}

fn remaining_hybrid(cost: &ManaCost) -> Vec<HybridSymbolView> {
    let mut out = Vec::new();
    for (a, b) in cost.hybrid.iter().flatten() {
        out.push(HybridSymbolView { colors: [a, b].iter().filter_map(|m| m.color()).map(color_view).collect(), life_cost: 0 });
    }
    for (i, &n) in cost.phyrexian.iter().enumerate() {
        for _ in 0..n {
            out.push(HybridSymbolView { colors: vec![color_view(Color::ALL[i])], life_cost: 2 });
        }
    }
    out
}

fn purpose_view(purpose: Purpose, opening: bool) -> CardChoicePurposeView {
    use CardChoicePurposeView as V;
    match purpose {
        Purpose::Band => V::Band,
        Purpose::Discard => {
            if opening {
                V::PutOnBottom
            } else {
                V::Discard
            }
        }
        Purpose::TakeFromHand => V::Discard,
        Purpose::Sacrifice | Purpose::SacrificeCost => V::Sacrifice,
        Purpose::ReturnToHand => V::ReturnToHand,
        Purpose::PutOnLibrary | Purpose::PutOnTop => V::PutOnTop,
        Purpose::PutOntoBattlefield => V::PutOntoBattlefield,
        Purpose::KeepLegend => V::KeepLegend,
        Purpose::Exile => V::Exile,
        Purpose::Untap => V::Untap,
        Purpose::Tap => V::Tap,
        Purpose::ExileToPay => V::ExileToPay,
        Purpose::Keep => V::Keep,
        Purpose::PutIntoHand => V::PutIntoHand,
        Purpose::Cast => V::Cast,
        Purpose::CastFree => V::CastFree,
        Purpose::CastCopyFree | Purpose::CastCopyFreeWithin(_) => V::CastCopyFree,
        Purpose::TapToPay => V::TapToPay,
        Purpose::OrderSimultaneous => V::OrderSimultaneous,
        Purpose::Destroy => V::Destroy,
        Purpose::Vote => V::Vote,
        Purpose::PutIntoGraveyard => V::PutIntoGraveyard,
        Purpose::Play => V::Play,
        Purpose::Splice => V::Splice,
        Purpose::PutOnBottom => V::PutOnBottom,
        Purpose::Attach => V::Attach,
        Purpose::Encode => V::Encode,
        Purpose::Behold => V::Behold,
        Purpose::Blight => V::Blight,
        Purpose::TapToReduce => V::TapToReduce,
        Purpose::Reveal => V::Reveal,
        Purpose::Plot => V::Plot,
        Purpose::PutCounters(..) => V::PutCounters,
        Purpose::PreventDamageFrom(_) => V::PreventDamageFrom,
        Purpose::PayLifeOrPutOnTop(_) => V::PayLifeOrPutOnTop,
        Purpose::SeparatePiles
        | Purpose::ChoosePile
        | Purpose::CopyAsEnters
        | Purpose::CopyWhich
        | Purpose::Targets
        | Purpose::ChangeTargetTo
        | Purpose::EnterAttacking
        | Purpose::Color
        | Purpose::Optional
        | Purpose::Ask(_)
        | Purpose::EndTurn
        | Purpose::RevealForMiracle(_)
        | Purpose::CastForMiracleCost(_)
        | Purpose::PromiseGift(_)
        | Purpose::CastTransformedFree
        | Purpose::CastForMadness
        | Purpose::CastFoundOrHand
        | Purpose::CastOrBottom
        | Purpose::AlternativeCost => V::Keep,
    }
}

/// The costs a spell's card may ask a keyword action for: its optional cost, its additional cost, and each of its choices.
fn costs_of(game: &Game, source: ObjectId) -> Vec<Cost> {
    let def = game.cards.get(game.state.obj(source).card);
    def.optional_cost.map(|o| o.cost).into_iter().chain(def.additional_cost).chain(def.additional_choice.iter().map(|c| c.cost)).collect()
}

/// "Behold" or "Blight" for an additional cost that is that keyword action, else nothing.
fn cost_kind(game: &Game, source: ObjectId) -> &'static str {
    let costs = costs_of(game, source);
    if costs.iter().any(|c| c.behold.is_some()) {
        "Behold"
    } else if costs.iter().any(|c| c.blight > 0) {
        "Blight"
    } else {
        ""
    }
}

/// The quality a spell's behold cost names ("Dragon").
fn beheld_quality(game: &Game, source: ObjectId) -> &'static str {
    costs_of(game, source).iter().find_map(|c| c.behold.map(|(_, _, quality)| quality)).unwrap_or("")
}

/// The verb of a "you may" and its amount (`CastDuringResolution` with an amount of 1 is a cast without paying).
fn may_verb(purpose: Purpose) -> (String, i32) {
    let verb = match purpose {
        Purpose::Cast => "CastDuringResolution",
        Purpose::CastFree => return ("CastDuringResolution".into(), 1),
        Purpose::CastCopyFree | Purpose::CastCopyFreeWithin(_) => "CastCopyWithoutPaying",
        Purpose::CastTransformedFree => "CastTransformed",
        Purpose::CastForMadness => "CastForMadnessCost",
        Purpose::CastFoundOrHand => "CastFoundOrPutIntoHand",
        Purpose::CastOrBottom => "CastWithoutPaying",
        Purpose::CastForMiracleCost(_) => "CastForMiracleCost",
        Purpose::PromiseGift(gift) => match gift {
            Gift::Food => "PromiseGiftFood",
            Gift::Card => "PromiseGiftCard",
            Gift::TappedFish => "PromiseGiftTappedFish",
            Gift::ExtraTurn => "PromiseGiftExtraTurn",
            Gift::Treasure => "PromiseGiftTreasure",
            Gift::Octopus => "PromiseGiftOctopus",
        },
        Purpose::CopyAsEnters => "CopyAsEnters",
        other => return (format!("{other:?}"), 0),
    };
    (verb.into(), 0)
}

/// The modal spell or ability whose modes `modes` are the labels of.
fn modal_of(game: &Game, modes: &[Label]) -> Option<&'static ModalDef> {
    let Some(Label::Mode(ability, _)) = modes.first() else { return None };
    let def = game.cards.get(ability.card);
    if ability.index == u8::MAX {
        def.modal
    } else {
        match def.abilities.get(ability.index as usize) {
            Some(AbilityDef::Activated(a)) => a.modal,
            Some(AbilityDef::Triggered(t)) => t.modal,
            _ => None,
        }
    }
}

/// What a choice of modes says beyond its heading, and whether choosing more of them than the fewest costs something: a
/// modal's extra modes cost their escalate cost or their own cost (spree, CR 702.172a), the faces of a card are not modes
/// at all, and the labels of a pile or a plain choice are chosen without a target.
/// What a target holds, in words: an object's name, or the seat of a player.
fn target_words(game: &Game, target: Target) -> String {
    match target {
        Target::Object(id) => game.cards.get(game.chars(id).card).name.to_string(),
        Target::Player(p) => format!("Player {}", seat(p)),
        _ => String::new(),
    }
}

fn modes_detail(game: &Game, modes: &[Label], faces: bool) -> (String, bool) {
    if faces && modes.iter().any(|l| matches!(l, Label::Fused(_))) {
        return ("Only the face you choose is cast, with its own cost and mana value; fused, both halves are, for both costs".into(), false);
    }
    if faces {
        return ("Only the face you choose is cast, with its own cost and mana value".into(), false);
    }
    match modal_of(game, modes) {
        Some(modal) => {
            if let Some(escalate) = modal.escalate {
                (format!("Each mode past the first costs {}", cost_text_of(&escalate, "")), true)
            } else if modal.modes.iter().any(|m| m.cost.mana_value() > 0) {
                ("Each mode adds the cost shown with it".into(), true)
            } else {
                (String::new(), false)
            }
        }
        None if modes.iter().all(|l| matches!(l, Label::Card(..) | Label::Option(..) | Label::Name(_) | Label::Pile { .. })) => ("Chosen, not targeted".into(), false),
        None => (String::new(), false),
    }
}

/// A miracle cost as words: its symbols, or that it can't be paid (CR 118.6).
fn cost_words(cost: Option<ManaCost>) -> String {
    cost.map_or_else(|| "(unpayable: it has no mana cost)".to_string(), |c| c.to_string())
}

/// What a miracle ability costs, in words, from the card table alone (the card being drawn is no object yet when it is
/// asked about), reduced as its grant says: a split card's for each half it may be cast as, since a granted cost may be
/// read off the half (CR 709.3a), and what of a reduction is left to come off X (CR 118.7a).
fn miracle_cost_words(game: &Game, miracle: MiracleOf) -> String {
    let words = |face: CardId| match miracle_cost_and_x_less(game, miracle, face) {
        Some((cost, x_less)) if x_less > 0 => format!("{cost} (X less {x_less})"),
        cost => cost_words(cost.map(|(c, _)| c)),
    };
    match game.cards.get(miracle.card).split {
        Some((left, right)) if !miracle.is_own() => {
            [left, right].iter().map(|half| format!("{} {}", half.name, words(game.cards.id_of(half)))).collect::<Vec<_>>().join(" or ")
        }
        _ => words(miracle.card),
    }
}

/// One miracle ability of a card, for the choice of which to reveal it with (CR 702.94a, 607.5): its cost and whose it is.
fn miracle_label(game: &Game, miracle: MiracleOf) -> String {
    let cost = miracle_cost_words(game, miracle);
    if miracle.is_own() { format!("Miracle {cost}, its own") } else { format!("Miracle {cost}, from {}", game.cards.get(miracle.granter).name) }
}

/// A miracle ability as the questions about it word it: "miracle {2}", and who gave it, if not the card itself.
fn miracle_words(game: &Game, miracle: MiracleOf) -> (String, String) {
    let cost = format!("miracle {}", miracle_cost_words(game, miracle));
    let from = if miracle.is_own() { String::new() } else { format!("{} gives it {cost}", game.cards.get(miracle.granter).name) };
    (cost, from)
}

/// Miracle's two questions (CR 702.94a), worded with the cost: whether to reveal the card being drawn, and, as the linked
/// ability resolves, whether to cast it for the cost of the miracle ability it was revealed with. A cost a permanent gave
/// is not on the card's face, so the question says where it comes from.
fn miracle_prompt(game: &Game, player: i32, handle: ViewHandle, purpose: Purpose) -> MayPrompt {
    let seen = "Every player sees it from now on, even if you then decide not to cast it";
    let (verb, question, detail, yes, no, no_default) = match purpose {
        Purpose::RevealForMiracle(Some(one)) => {
            let (cost, from) = miracle_words(game, one);
            let detail = if from.is_empty() { seen.to_string() } else { format!("{from}  ·  {seen}") };
            ("RevealForMiracle", format!("Reveal {{source}} for its {cost}?"), detail, "Reveal", "Keep it hidden", true)
        }
        Purpose::RevealForMiracle(None) => (
            "RevealForMiracle",
            "Reveal {source} for one of its miracle abilities?".to_string(),
            format!("It has more than one: you choose which next  ·  {seen}"),
            "Reveal",
            "Keep it hidden",
            true,
        ),
        Purpose::CastForMiracleCost(miracle) => {
            let (cost, from) = miracle_words(game, miracle);
            let pay = format!("You pay its {cost} instead of its mana cost");
            let detail = if from.is_empty() { pay } else { format!("{pay}  ·  {from}") };
            ("CastForMiracleCost", format!("Cast {{source}} for its {cost}?"), detail, "Cast", "Don't cast", false)
        }
        _ => unreachable!("only miracle's questions are worded here"),
    };
    MayPrompt { player, source: handle, verb: verb.into(), amount: 0, question, detail, yes: yes.into(), no: no.into(), no_default }
}

/// What a card of the request is cast as, for the face choice: its name and its cost.
fn face_text(game: &Game, face: CardId) -> String {
    let def = game.cards.get(face);
    if def.has_cost { format!("{} {}", def.name, def.cost) } else { def.name.to_string() }
}

/// Damage that replacement effects are about to change, in words for the player who orders them (CR 616.1): how much,
/// from what and to whom as the effects applied so far have left it ("3 damage from Lightning Bolt to you").
fn damage_words(game: &Game, asked: PlayerId, d: DamageAbout) -> String {
    let to = match d.to {
        Target::Player(p) if p == asked => "you".to_string(),
        Target::Player(_) => "your opponent".to_string(),
        Target::Object(id) => game.name(id).to_string(),
        _ => "nothing".to_string(),
    };
    format!("{} damage from {} to {to}", d.amount, game.name(d.source))
}

/// Counters about to be put on a permanent, in words: "2 +1/+1 counters would be put on Walking Ballista".
fn counter_words(game: &Game, subject: Option<ObjectId>, counters: &[(CounterKind, i16)]) -> String {
    let kinds: Vec<String> = counters
        .iter()
        .map(|&(kind, n)| format!("{n} {} counter{}", kind.name(), if n == 1 { "" } else { "s" }))
        .collect();
    let on = subject.map_or(ObjectName::Literal("it"), |s| game.name(s));
    format!("{} would be put on {on}", kinds.join(" and "))
}

fn handles_of(handles: &mut HandleSpace, ids: &[ObjectId]) -> Vec<ViewHandle> {
    ids.iter().map(|&id| handles.handle(id)).collect()
}

/// Blocks, (blocker, attacker), as the wire names them.
fn blocks(handles: &mut HandleSpace, pairs: &[(ObjectId, ObjectId)]) -> Vec<BlockMessage> {
    pairs.iter().map(|&(blocker, attacker)| BlockMessage { blocker: handles.handle(blocker), attacker: handles.handle(attacker) }).collect()
}

/// The prompt a request is, and whether the seat may give the announcement up.
pub fn prompt(game: &Game, request: &Request, handles: &mut HandleSpace) -> Result<(PromptPayload, bool), String> {
    let player = seat(request.player);
    let opening = game.state.agenda.last().is_some_and(|f| f.kind == FrameKind::Opening);
    let payload = match &request.kind {
        RequestKind::Priority { options } => {
            PromptPayload::PriorityPromptMessage(PriorityPrompt { player, options: options_messages(game, handles, options) })
        }
        RequestKind::PayCosts { source, total, remaining, options, announcing, x, most, loyalty, attackers, life } => {
            let def = game.cards.get(game.state.obj(*source).card);
            let kicker = def.kicker;
            let spends_no_mana = game.chars(*source).has(Keyword::CantSpendManaToCast);
            // "You may sacrifice any number of ... This spell costs {1} less for each" (CR 601.2b): what the number announced is.
            let sacrifices = if *announcing { sacrifices_for_reduction(game, *source, request.player) } else { None };
            PromptPayload::PayCostsPromptMessage(PayCostsPrompt {
                player,
                spell: source_handle(game, handles, *source),
                total: total.to_string(),
                remaining: remaining.to_string(),
                remaining_generic: i32::from(remaining.generic),
                remaining_colored: remaining_colored(remaining),
                remaining_colorless: i32::from(remaining.symbols[Mana::C as usize]),
                remaining_hybrid: remaining_hybrid(remaining),
                options: options_messages(game, handles, options),
                has_x: total.x > 0 || (*announcing && kicker.is_some()),
                x: wire(*x),
                kicker: kicker.map_or(String::new(), |k| k.cost.to_string()),
                multikicker: kicker.is_some_and(|k| k.multi),
                kicker_paid: kicker.is_some() && *x > 0,
                announces_x_or_kicks: *announcing,
                // A "−X" loyalty cost reaches as far as the loyalty there is (CR 606.4, 118.3); mana is not asked.
                most_affordable: if *loyalty {
                    i32::from(game.state.obj(*source).counters(CounterKind::Loyalty).max(0))
                } else if let Some((_, could)) = sacrifices {
                    could.min(i32::MAX as usize) as i32
                } else {
                    -1
                },
                sacrifice_reduces: sacrifices.map_or(0, |(each, _)| i32::from(each)),
                most_announced: wire(*most),
                x_removes_loyalty: *loyalty,
                spends_no_mana,
                attackers: handles_of(handles, attackers),
                life: (*life).min(i32::MAX as u32) as i32,
                ..Default::default()
            })
        }
        RequestKind::UnlessPays { source, subject, total, remaining, life, life_payable, question, options, x } => PromptPayload::UnlessPaysPromptMessage(UnlessPaysPrompt {
            player,
            source: source_handle(game, handles, *source),
            subject: source_handle(game, handles, *subject),
            total: total.to_string(),
            remaining: remaining.to_string(),
            remaining_generic: i32::from(remaining.generic),
            remaining_colored: remaining_colored(remaining),
            remaining_hybrid: remaining_hybrid(remaining),
            options: options_messages(game, handles, options),
            verb: question.verb.into(),
            life: i32::from(*life),
            life_short: !*life_payable,
            amount: i32::from(question.amount),
            question: question.ask.into(),
            detail: question.detail.into(),
            yes: question.yes.into(),
            no: question.no.into(),
            // "You may pay {X}" (CR 107.3f): X is chosen as it is paid, raised and lowered by the options.
            choosing_x: x.is_some(),
            x: x.map_or(0, i32::from),
        }),
        RequestKind::ChooseCards { source, purpose, cards, rule } => {
            let kind = &rule.kind;
            let verdict = verdict_view(game, request, handles);
            // A rule that keeps a running total of mana value (CR 202.3): its most, and each card's mana value, read where it is
            // (an {X} in a library or graveyard is 0, CR 202.3e), index for index with the cards.
            let tally = rule.parts().find(|p| p.tally == cantrip_engine::select::Tally::ManaValueAtMost).map(|p| p.n);
            let owner = if *purpose == Purpose::TakeFromHand {
                cards.first().map_or(request.player, |c| game.state.obj(*c).owner)
            } else {
                request.player
            };
            PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
                player,
                owner: seat(owner),
                source: source_handle(game, handles, *source),
                purpose: purpose_view(*purpose, opening),
                cards: handles_of(handles, cards),
                count: count_shown(rule.count, &verdict),
                count_kind: count_kind_view(rule.count),
                words: rule.words(&|id| game.cards.get(CardId(id as u16)).name.to_string()),
                mana_values: if tally.is_some() { cards.iter().map(|&c| wire(game.mana_value(c))).collect() } else { Vec::new() },
                maximum_total_mana_value: tally.map(wire),
                // Blight (CR 701.68a) is a creature you control; behold (CR 701.4a) names its quality ("Dragon"): the
                // client says so in its caption. Any other choice is worded as the asking card words it ("nonland"), if it does.
                kind: match purpose {
                    Purpose::Blight => "creature".into(),
                    Purpose::Behold if !kind.is_empty() => (*kind).to_string(),
                    Purpose::Behold => beheld_quality(game, *source).into(),
                    _ => (*kind).to_string(),
                },
                maximum_mana_value: match purpose {
                    Purpose::CastCopyFreeWithin(left) => i32::from(*left),
                    _ => 0,
                },
                // The counters the chosen permanent gets, for the caption ("put 2 loyalty counters on").
                counter: match purpose {
                    Purpose::PutCounters(counter, _) => counter.name().into(),
                    _ => String::new(),
                },
                counter_count: match purpose {
                    Purpose::PutCounters(_, n) => i32::from(*n),
                    _ => 0,
                },
                prevent_left: match purpose {
                    Purpose::PreventDamageFrom(n) => i32::from(*n),
                    _ => 0,
                },
                // A band may have at most one creature without banding (CR 702.22c): which of the ones offered those are.
                without_banding: match purpose {
                    Purpose::Band => handles_of(
                        handles,
                        &cards.iter().copied().filter(|&c| !game.chars(c).has(Keyword::Banding)).collect::<Vec<_>>(),
                    ),
                    _ => Vec::new(),
                },
                verdict,
                life: match purpose {
                    Purpose::PayLifeOrPutOnTop(n) => i32::from(*n),
                    _ => 0,
                },
                ..Default::default()
            })
        }
        RequestKind::TapForPower { source, purpose, cards, powers, minimum } => {
            // Harmonize's creature (CR 702.180a) is up to one, and `powers` is what tapping each takes off the cost.
            let reduce = *purpose == Purpose::TapToReduce;
            PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
                player,
                owner: player,
                source: source_handle(game, handles, *source),
                purpose: if reduce { CardChoicePurposeView::TapToReduce } else { CardChoicePurposeView::TapToPay },
                cards: handles_of(handles, cards),
                verdict: verdict_view(game, request, handles),
                count: if reduce { 1 } else { cards.len() as i32 },
                kind: if reduce { "creature".into() } else { String::new() },
                count_kind: CountKindView::UpTo,
                minimum_power: i32::from(*minimum),
                // Signed: a negative power takes from the total (CR 107.1b).
                powers: powers.iter().map(|&p| i32::from(p)).collect(),
                ..Default::default()
            })
        }
        RequestKind::CollectEvidence { source, cards, values, minimum } => {
            PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
                player,
                owner: player,
                source: source_handle(game, handles, *source),
                purpose: CardChoicePurposeView::ExileToPay,
                cards: handles_of(handles, cards),
                verdict: verdict_view(game, request, handles),
                count: cards.len() as i32,
                count_kind: CountKindView::UpTo,
                minimum_mana_value: i32::from(*minimum),
                mana_values: values.iter().map(|&v| i32::from(v)).collect(),
                ..Default::default()
            })
        }
        // Craft's materials (CR 702.167a): permanents and graveyard cards together, of the quality the ability names; an exact
        // number, or at least so many ("one or more").
        RequestKind::Craft { source, cards, minimum, maximum, quality } => {
            PromptPayload::ChooseCardsPromptMessage(ChooseCardsPrompt {
                player,
                owner: player,
                source: source_handle(game, handles, *source),
                purpose: CardChoicePurposeView::Craft,
                cards: handles_of(handles, cards),
                verdict: verdict_view(game, request, handles),
                count: wire(*maximum),
                count_kind: if minimum < maximum { CountKindView::UpTo } else { CountKindView::Exactly },
                minimum: if minimum < maximum { wire(*minimum) } else { 0 },
                kind: (*quality).to_string(),
                ..Default::default()
            })
        }
        RequestKind::ChooseTarget { source, purpose, candidates, optional, least, count, any_number, label, slot_player, total } => {
            let source_handle = source_handle(game, handles, *source);
            // Each candidate the wire can carry, with its mana value when the targets are limited to a total of them.
            let sent: Vec<(TargetMessage, i32)> = candidates
                .iter()
                .enumerate()
                .filter_map(|(i, t)| {
                    let value = total.as_ref().map_or(0, |t| wire(t.values.get(i).copied().unwrap_or(0)));
                    target_message(handles, *t).map(|m| (m, value))
                })
                .collect();
            PromptPayload::ChooseTargetPromptMessage(ChooseTargetPrompt {
                player,
                ability: source_handle,
                source: source_handle,
                text: (*label).to_string(),
                // "With total mana value X or less" (CR 601.2c, 202.3): each candidate's mana value, index for index, and the
                // most the targets chosen may add up to.
                mana_values: if total.is_some() { sent.iter().map(|&(_, v)| v).collect() } else { Vec::new() },
                maximum_total_mana_value: total.as_ref().map(|t| wire(t.most)),
                candidates: sent.into_iter().map(|(m, _)| m).collect(),
                optional: *optional,
                count: wire(*count),
                targeted: *purpose == Purpose::Targets,
                // Only a printed range has a minimum (the wire's zero is "exactly count, or none where optional").
                minimum: if *least > 0 && least < count { wire(*least) } else { 0 },
                any_number: *any_number,
                for_player: slot_player.map_or(0, seat),
            })
        }
        RequestKind::ChooseModes { source, modes, eligible, count, at_most, affordable, additional_cost, repeat } => {
            // The faces a card may be cast as (CR 709.3): not modes, so each is its name and its cost, and the question says so.
            let faces = !modes.is_empty() && modes.iter().all(|l| matches!(l, Label::Face(_) | Label::Fused(_)));
            let (mut detail, extra_modes_cost) = modes_detail(game, modes, faces);
            if matches!(modes.first(), Some(Label::TargetSlot(..))) {
                detail = format!("That target becomes {}", game.cards.get(game.chars(*source).card).name);
            }
            PromptPayload::ChooseModesPromptMessage(ChooseModesPrompt {
                player,
                source: source_handle(game, handles, *source),
                modes: modes
                    .iter()
                    .map(|l| match l {
                        Label::Face(face) => face_text(game, *face),
                        // Both halves at once, for both mana costs (CR 702.102a, 702.102c).
                        Label::Fused(card) => format!("Fuse: both halves {}", game.cards.get(*card).cost),
                        Label::Miracle(m) => miracle_label(game, *m),
                        // The slot's text and what it holds now, so two slots worded alike are told apart.
                        Label::TargetSlot(text, held) => format!("{text}: {}", target_words(game, *held)),
                        other => label_text(other).to_string(),
                    })
                    .collect(),
                eligible: eligible.iter().map(|&m| i32::from(m)).collect(),
                count: i32::from(*count),
                may_repeat: *repeat,
                additional_cost: *additional_cost,
                heading: if faces {
                    "Choose which face to cast".into()
                } else if matches!(modes.first(), Some(Label::Miracle(..))) {
                    "Choose which miracle to reveal it with".into()
                } else if matches!(modes.first(), Some(Label::ManaType(..))) {
                    "Choose which type of mana to add".into()
                } else if matches!(modes.first(), Some(Label::ManaCost)) {
                    "Choose how to pay for it".into()
                } else if matches!(modes.first(), Some(Label::TargetSlot(..))) {
                    // Two instances of "target" hold the same object (CR 115.3): which one changes to the source (CR 115.7b).
                    format!("Choose which target to change to {}", game.cards.get(game.chars(*source).card).name)
                } else if let Some(heading) = options_heading(game, modes) {
                    heading
                } else {
                    String::new()
                },
                at_most: i32::from(*at_most),
                affordable: affordable.iter().map(|&m| i32::from(m)).collect(),
                cost_kind: if *additional_cost { cost_kind(game, *source).into() } else { String::new() },
                detail,
                extra_modes_cost,
                texts: modes
                    .iter()
                    .filter_map(|l| match l {
                        Label::Face(face) | Label::Fused(face) => Some(game.cards.get(*face).text.to_string()),
                        _ => None,
                    })
                    .collect(),
            })
        }
        RequestKind::ChooseSubtype { source, choices, purpose } => PromptPayload::ChooseModesPromptMessage(ChooseModesPrompt {
            player,
            source: source_handle(game, handles, *source),
            modes: choices.iter().map(|s| s.name().to_string()).collect(),
            eligible: (0..choices.len() as i32).collect(),
            count: 1,
            at_most: 1,
            heading: subtype_heading(game.cards.get(game.chars(*source).card).name, choices, *purpose),
            detail: match purpose {
                WordUse::Chosen => String::new(),
                WordUse::Replaced => "Every instance of it in the target's text is replaced".into(),
                WordUse::Replacing => "It replaces every instance of the first in the target's text".into(),
            },
            ..Default::default()
        }),
        RequestKind::ChooseColors { source, choices, count, purpose } => PromptPayload::ChooseColorsPromptMessage(ChooseColorsPrompt {
            player,
            source: source_handle(game, handles, *source),
            choices: colors_view(*choices),
            count: i32::from(*count),
            may_repeat: i32::from(*count) > choices.iter().count() as i32,
            verb: match purpose {
                ColorUse::Any => String::new(),
                ColorUse::AddMana => "AddMana".into(),
                ColorUse::Benefit => "Benefit".into(),
                ColorUse::ReplacedWord => "ReplaceWord".into(),
                ColorUse::ReplacingWord => "ReplacingWord".into(),
            },
        }),
        RequestKind::ChooseName { source, looked_at, suggestion, kind } => {
            let mut names: Vec<String> = game
                .cards
                .cards
                .iter()
                .zip(&game.cards.card_named)
                // Only a card's name is listed: a token's or an emblem's is not one unless a card has it too (CR 201.4).
                .filter(|&(_, &named)| named)
                .map(|(c, _)| c)
                // A split card has two names and a player names one of them (CR 709.4a): its halves are listed, not the whole.
                .filter(|c| c.split.is_none() && kind.allows(c.is(CardType::Land)))
                .map(|c| c.name.to_string())
                .collect();
            names.sort();
            names.dedup();
            let owner = looked_at.first().map_or(0, |c| seat(game.state.obj(*c).owner));
            PromptPayload::NameCardPromptMessage(NameCardPrompt {
                player,
                source: source_handle(game, handles, *source),
                owner,
                hand: handles_of(handles, looked_at),
                suggestion: suggestion.map_or_else(|| names.first().cloned().unwrap_or_default(), |c| game.cards.get(c).name.to_string()),
                names,
                kind: kind.word().to_string(),
            })
        }
        RequestKind::May { source, purpose: Purpose::Ask(question) } => PromptPayload::MayPromptMessage(MayPrompt {
            player,
            source: source_handle(game, handles, *source),
            verb: question.verb.into(),
            amount: i32::from(question.amount),
            question: question.ask.into(),
            detail: question.detail.into(),
            yes: question.yes.into(),
            no: question.no.into(),
            no_default: !question.yes_is_default,
        }),
        RequestKind::May { source, purpose: purpose @ (Purpose::RevealForMiracle(_) | Purpose::CastForMiracleCost(_)) } => {
            PromptPayload::MayPromptMessage(miracle_prompt(game, player, source_handle(game, handles, *source), *purpose))
        }
        RequestKind::May { source, purpose } => {
            let (verb, amount) = may_verb(*purpose);
            PromptPayload::MayPromptMessage(MayPrompt { player, source: source_handle(game, handles, *source), verb, amount, ..MayPrompt::default() })
        }
        RequestKind::ChoosePlayer { source, candidates, verb, amount } => PromptPayload::ChoosePlayerPromptMessage(ChoosePlayerPrompt {
            player,
            source: source_handle(game, handles, *source),
            candidates: candidates.iter().map(|&p| seat(p)).collect(),
            verb: (*verb).to_string(),
            amount: i32::from(*amount),
        }),
        // Counters removed from among permanents (CR 118.3, 122.1) - as a cost (Dawnhand Dissident's casts) or as an effect
        // resolves (Dyadrine's "a +1/+1 counter from each of two creatures", a pool of one each; Heartless Act's "up to three
        // counters", `up_to`, CR 608.2d): each kind on each permanent, named as the counters on the table are.
        RequestKind::RemoveCounters { source, pools, total, up_to } => PromptPayload::RemoveCountersPromptMessage(RemoveCountersPrompt {
            player,
            source: source_handle(game, handles, *source),
            total: i32::from(*total),
            up_to: *up_to,
            pools: pools
                .iter()
                .map(|p| CounterPoolMessage { object: handles.handle(p.object), counter: p.kind.name().to_string(), count: i32::from(p.count) })
                .collect(),
        }),
        RequestKind::ChooseDividedTargets { source, candidates, total, max_targets, optional } => {
            let source_handle = source_handle(game, handles, *source);
            PromptPayload::ChooseDividedTargetsPromptMessage(ChooseDividedTargetsPrompt {
                player,
                ability: source_handle,
                source: source_handle,
                text: String::new(),
                candidates: candidates.iter().filter_map(|t| target_message(handles, *t)).collect(),
                total: wire(*total),
                max_targets: wire(*max_targets),
                optional: *optional,
            })
        }
        RequestKind::DeclareAttackers { eligible, players, planeswalkers, battles, requirements, costs, can_pay } => {
            PromptPayload::DeclareAttackersPromptMessage(DeclareAttackersPrompt {
                player,
                eligible: handles_of(handles, eligible),
                attackable_players: players.iter().map(|&p| seat(p)).collect(),
                attackable_planeswalkers: handles_of(handles, planeswalkers),
                attackable_battles: handles_of(handles, battles),
                requirements: requirements
                    .iter()
                    .map(|&a| AttackRequirementMessage { attacker: handles.handle(a), players: Vec::new() })
                    .collect(),
                forbidden: Vec::new(),
                costs: costs
                    .iter()
                    .map(|c| AttackCostMessage {
                        attacker: handles.handle(c.attacker),
                        target_player: c.target.player().map_or(0, seat),
                        target_permanent: c.target.object().map_or(ViewHandle::NONE, |id| handles.handle(id)),
                        cost: c.cost.to_string(),
                        mana_value: c.cost.mana_value() as i32,
                        generic: i32::from(c.cost.generic),
                    })
                    .collect(),
                can_pay: if *can_pay == u16::MAX { -1 } else { i32::from(*can_pay) },
            })
        }
        RequestKind::AssignCombatDamage { divisions } => PromptPayload::AssignCombatDamagePromptMessage(AssignCombatDamagePrompt {
            player,
            divisions: divisions
                .iter()
                .map(|d| DamageDivisionMessage {
                    source: handles.handle(d.source),
                    damage: i32::from(d.damage),
                    creatures: handles_of(handles, &d.creatures),
                    lethal: d.lethal.iter().map(|&l| i32::from(l)).collect(),
                    may_assign_to_defender: d.may_assign_to_defender,
                    defender: d.defender.player().map_or(0, seat),
                    suggested: d.suggested.iter().map(|&l| i32::from(l)).collect(),
                    suggested_to_defender: i32::from(d.suggested_to_defender),
                    defender_permanent: d.defender.object().map_or(ViewHandle::NONE, |id| handles.handle(id)),
                    banding: d.banding,
                })
                .collect(),
        }),
        RequestKind::DeclareBlockers { eligible, attackers, allowed, requirements } => {
            PromptPayload::DeclareBlockersPromptMessage(DeclareBlockersPrompt {
                player,
                eligible: handles_of(handles, eligible),
                attackers: handles_of(handles, attackers),
                allowed: allowed
                    .iter()
                    .map(|&(blocker, attacker)| BlockMessage { blocker: handles.handle(blocker), attacker: handles.handle(attacker) })
                    .collect(),
                minimum_blockers: attackers.iter().map(|&a| wire(count_u32(game.chars(a).minimum_blockers()))).collect(),
                maximum_blockers: attackers
                    .iter()
                    .map(|&a| match game.chars(a).maximum_blockers() {
                        usize::MAX => 0,
                        most => wire(count_u32(most)),
                    })
                    .collect(),
                requirements: blocks(handles, &requirements.pairs),
                most_requirements: wire(requirements.most),
                suggested: blocks(handles, &requirements.suggested),
                bands: attackers
                    .iter()
                    .map(|a| requirements.bands.iter().find(|b| b.0 == *a).map_or(0, |b| b.1 as i32))
                    .collect(),
            })
        }
        RequestKind::DiscardToHandSize { count, hand } => PromptPayload::DiscardPromptMessage(DiscardPrompt {
            player,
            count: wire(*count),
            maximum_hand_size: 7,
            hand: handles_of(handles, hand),
        }),
        RequestKind::Mulligan { mulligans } => PromptPayload::MulliganPromptMessage(MulliganPrompt {
            player,
            mulligans: i32::from(*mulligans),
            keeps: 7 - i32::from(*mulligans),
        }),
        RequestKind::ChooseStartingPlayer { candidates } => {
            PromptPayload::ChooseStartingPlayerPromptMessage(ChooseStartingPlayerPrompt {
                player,
                candidates: candidates.iter().map(|&p| seat(p)).collect(),
            })
        }
        RequestKind::OrderTriggers { sources, abilities } => PromptPayload::OrderTriggersPromptMessage(OrderTriggersPrompt {
            player,
            sources: sources.iter().map(|&s| source_handle(game, handles, s)).collect(),
            texts: abilities.iter().map(|&a| game.cards.ability(a).text().to_string()).collect(),
        }),
        RequestKind::DistributeCards { source, cards, destinations, may_shuffle } => {
            PromptPayload::DistributeCardsPromptMessage(DistributeCardsPrompt {
                player,
                source: source_handle(game, handles, *source),
                cards: handles_of(handles, cards),
                destinations: destinations
                    .iter()
                    .map(|d| CardDestinationMessage {
                        kind: match d.kind {
                            DestinationKind::LibraryTop => CardDestinationKindView::LibraryTop,
                            DestinationKind::LibraryBottom => CardDestinationKindView::LibraryBottom,
                            DestinationKind::Graveyard => CardDestinationKindView::Graveyard,
                            DestinationKind::Hand => CardDestinationKindView::Hand,
                            DestinationKind::Exile => CardDestinationKindView::Exile,
                            DestinationKind::BattlefieldFaceDown => CardDestinationKindView::BattlefieldFaceDown,
                        },
                        owner: player,
                        min: i32::from(d.min),
                        max: if d.max == u8::MAX { i32::MAX } else { i32::from(d.max) },
                        eligible: handles_of(handles, &d.eligible),
                        ordered: matches!(d.kind, DestinationKind::LibraryTop | DestinationKind::LibraryBottom),
                        randomized: false,
                        groups: Vec::new(),
                        group_names: Vec::new(),
                    })
                    .collect(),
                may_shuffle: *may_shuffle,
            })
        }
        RequestKind::SearchLibrary { source, library, rule, destination } => {
            let _ = source;
            // Whose library it is: the cards in it are all its owner's.
            let owner = library.first().map_or(request.player, |c| game.state.obj(*c).owner);
            let verdict = verdict_view(game, request, handles);
            PromptPayload::SearchLibraryPromptMessage(SearchLibraryPrompt {
                player,
                count: count_shown(rule.count, &verdict),
                count_kind: count_kind_view(rule.count),
                kind: rule.kind.to_string(),
                words: rule.words(&|id| game.cards.get(CardId(id as u16)).name.to_string()),
                verdict,
                library: handles_of(handles, library),
                destination: match destination {
                    SearchDestination::Hand => SearchDestinationView::Hand,
                    SearchDestination::TopOfLibrary => SearchDestinationView::TopOfLibrary,
                    SearchDestination::Graveyard => SearchDestinationView::Graveyard,
                    SearchDestination::Battlefield => SearchDestinationView::Battlefield,
                    SearchDestination::Exile => SearchDestinationView::Exile,
                    SearchDestination::ExileFaceDown => SearchDestinationView::ExileFaceDown,
                    SearchDestination::Cast => SearchDestinationView::Cast,
                    SearchDestination::Found => SearchDestinationView::Found,
                    SearchDestination::BattlefieldTapped => SearchDestinationView::BattlefieldTapped,
                    SearchDestination::CastFree => SearchDestinationView::CastFree,
                    SearchDestination::LandTappedElseHand => SearchDestinationView::LandTappedElseHand,
                },
                owner: seat(owner),
            })
        }
        RequestKind::ChooseReplacement { subject, life, sources, hints, damage, turn, counters, draw } => {
            PromptPayload::ChooseReplacementPromptMessage(ChooseReplacementPrompt {
                player,
                subject: subject.map_or(ViewHandle::NONE, |s| source_handle(game, handles, s)),
                // A change of life is asked of the player whose life it is (CR 616.1); damage, of the player it would be
                // dealt to or whose permanent it would be dealt to, as the effects so far have left it.
                event: match (life, damage) {
                    (Some(n), _) if *n > 0 => format!("You would gain {n} life"),
                    (Some(n), _) => format!("You would lose {} life", n.unsigned_abs()),
                    (None, Some(d)) => damage_words(game, request.player, *d),
                    // The turn effects would skip (CR 614.10) is the asked player's.
                    (None, None) if *turn => "Your turn would begin".to_string(),
                    // Counters about to be put on a permanent (CR 122.6), as the effects so far have left them.
                    (None, None) if !counters.is_empty() => counter_words(game, *subject, counters),
                    // A card about to be drawn (CR 121.6), asked of the player who would draw it (CR 616.1).
                    (None, None) if *draw => "You would draw a card".to_string(),
                    (None, None) => String::new(),
                },
                sources: sources.iter().map(|&s| source_handle(game, handles, s)).collect(),
                hints: hints.iter().map(|h| (*h).to_string()).collect(),
                source_names: sources.iter().map(|&s| source_name(game, request.player, s)).collect(),
            })
        }
    };
    Ok((payload, request.may_cancel))
}

/// What the rule of `request`, a question that picks a set of cards, says of `selected` (`selection_verdict`), as the seat
/// with `handles` sees it: whether it is an answer, and the cards that may be added. The prompt carries it for none chosen;
/// the host's `Check` answers it for any other selection.
pub fn verdict_of(game: &Game, request: &Request, selected: &[ObjectId], handles: &mut HandleSpace) -> Option<SelectionVerdictView> {
    let (cards, verdict) = selection_verdict(game, request, selected)?;
    Some(SelectionVerdictView { complete: verdict.complete, addable: verdict.addable.ones().map(|i| handles.handle(cards[i])).collect(), complete_alone: Vec::new() })
}

/// The prompt's first verdict (`verdict_of` of none chosen), and for a question that takes one card at most, which of the
/// cards that may be picked are an answer alone (`SelectionVerdictView::complete_alone`): the client sends such a pick at
/// once, where any other pick waits for the host's verdict on the cards chosen so far.
fn verdict_view(game: &Game, request: &Request, handles: &mut HandleSpace) -> SelectionVerdictView {
    let mut view = verdict_of(game, request, &[], handles).unwrap_or_default();
    if let RequestKind::ChooseCards { rule, .. } | RequestKind::SearchLibrary { rule, .. } = &request.kind {
        if rule.count.most() == Some(1) {
            let (cards, first) = selection_verdict(game, request, &[]).expect("a rule's question");
            for i in first.addable.ones() {
                if selection_verdict(game, request, &[cards[i]]).is_some_and(|(_, v)| v.complete) {
                    view.complete_alone.push(handles.handle(cards[i]));
                }
            }
        }
    }
    view
}

fn count_kind_view(count: Count) -> CountKindView {
    match count {
        Count::Exactly(_) => CountKindView::Exactly,
        Count::UpTo(_) => CountKindView::UpTo,
        Count::AnyNumber => CountKindView::AnyNumber,
        Count::AsManyAsCan(_) => CountKindView::AsManyAsCan,
    }
}

/// The number a caption says of a count: its own, for "as many as can be found" no more than can be, and for "any number"
/// every card that may be picked first.
fn count_shown(count: Count, verdict: &SelectionVerdictView) -> i32 {
    let found = count_u32(verdict.addable.len());
    match count {
        Count::AsManyAsCan(n) => wire(n.min(found)),
        _ => count.most().map_or(wire(found), wire),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game dealt these two decks, top of the library first, and the objects of the first seat's opening hand.
    fn game_with(first: &[&str]) -> (Game, Vec<ObjectId>) {
        let deck = |top: &[&str]| top.iter().map(|c| c.to_string()).chain(std::iter::repeat_n("Forest".to_string(), 20)).collect::<Vec<_>>();
        let game = deal(&[deck(first), deck(&[])], 1, false, None).expect("a game");
        let seat = game.state.seats().next().expect("a seat");
        let hand = game.state.zone(Zone::hand(seat)).to_vec();
        (game, hand)
    }

    /// A choice is worded as its card words it, not as the cards that happen to be eligible: a hand with only creatures left is
    /// still a choice of "a nonland card" when that is what the card says, and a card that says nothing says "card".
    #[test]
    fn a_choice_says_the_kind_its_card_gave_and_infers_none() {
        let (game, hand) = game_with(&["Llanowar Elves", "Ornithopter", "Forest"]);
        let creatures: Vec<ObjectId> = hand.iter().copied().filter(|&c| game.chars(c).is(CardType::Creature)).collect();
        assert_eq!(creatures.len(), 2, "the Elves and the Ornithopter");
        let seat = game.state.seats().next().expect("a seat");
        let kind_of = |kind: &'static str| {
            let request = Request {
                id: 1,
                player: seat,
                kind: RequestKind::choose_cards(hand[0], Purpose::Exile, hand.clone(), creatures.clone(), 1, false, kind),
                may_cancel: false,
            };
            match prompt(&game, &request, &mut HandleSpace::new()).expect("a prompt").0 {
                PromptPayload::ChooseCardsPromptMessage(p) => p.kind,
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(kind_of("nonland"), "nonland");
        assert_eq!(kind_of("artifact or creature"), "artifact or creature");
        // Only creatures were eligible, and nothing was said: nothing is claimed.
        assert_eq!(kind_of(""), "");
    }

    /// A part of a test rule that refuses an Ornithopter: what a card's own rule would be.
    fn no_ornithopter(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
        Verdict::each(choice, |card| q.chars(card).name != "Ornithopter")
    }

    /// A choice of cards reaches the client as its rule says it: the count and how it counts, the words its parts add to the
    /// caption, the running total a part keeps, and the rule's verdict on choosing nothing yet - the cards that may be picked first, which the rule's parts and
    /// its list decide together, and whether none at all is an answer.
    #[test]
    fn a_choice_sends_its_rule_as_count_words_and_first_verdict() {
        let (game, hand) = game_with(&["Llanowar Elves", "Ornithopter", "Forest"]);
        let seat = game.state.seats().next().expect("a seat");
        let part = cantrip_engine::select::tallied(&(no_ornithopter as SelectFn), 6, "with total mana value {n} or less", cantrip_engine::select::Tally::ManaValueAtMost);
        let rule = Selection::listed(Count::UpTo(2), &hand, hand[..2].to_vec()).called("creature").with(part);
        let request = Request {
            id: 1,
            player: seat,
            kind: RequestKind::ChooseCards { source: hand[0], purpose: Purpose::PutOntoBattlefield, cards: hand.clone(), rule: Box::new(rule) },
            may_cancel: false,
        };
        let mut handles = HandleSpace::new();
        let PromptPayload::ChooseCardsPromptMessage(p) = prompt(&game, &request, &mut handles).expect("a prompt").0 else { panic!("cards") };
        assert_eq!((p.count, p.count_kind, p.kind.as_str()), (2, CountKindView::UpTo, "creature"));
        assert_eq!(p.words, vec!["with total mana value 6 or less".to_string()]);
        // The running total it keeps (display alone): its most, and every card's mana value, index for index.
        assert_eq!((p.maximum_total_mana_value, p.mana_values.len()), (Some(6), hand.len()));
        assert_eq!(p.mana_values[..2], [1, 0], "the Elves 1, the Ornithopter 0");
        // The Elves alone: the Ornithopter is refused by the part, the Forest by the list.
        assert_eq!(p.verdict, SelectionVerdictView { complete: true, addable: vec![handles.handle(hand[0])], complete_alone: vec![] });
        assert_eq!(verdict_of(&game, &request, &[hand[0]], &mut handles).map(|v| (v.complete, v.addable.len())), Some((true, 0)));
    }

    /// A question that takes one card at most says in its prompt which cards are an answer alone, so the client sends such a
    /// pick without asking; a question of more cards says none.
    #[test]
    fn a_one_card_choice_marks_the_cards_that_are_an_answer_alone() {
        let (game, hand) = game_with(&["Llanowar Elves", "Ornithopter", "Forest"]);
        let seat = game.state.seats().next().expect("a seat");
        let ask = |rule: Selection| Request {
            id: 1,
            player: seat,
            kind: RequestKind::ChooseCards { source: hand[0], purpose: Purpose::PutOntoBattlefield, cards: hand.clone(), rule: Box::new(rule) },
            may_cancel: false,
        };
        let mut handles = HandleSpace::new();
        let PromptPayload::ChooseCardsPromptMessage(one) = prompt(&game, &ask(Selection::listed(Count::Exactly(1), &hand, hand[..2].to_vec())), &mut handles).expect("a prompt").0 else { panic!("cards") };
        assert_eq!(one.verdict.addable, vec![handles.handle(hand[0]), handles.handle(hand[1])]);
        assert_eq!(one.verdict.complete_alone, one.verdict.addable, "either of the two is the whole answer");
        let PromptPayload::ChooseCardsPromptMessage(two) = prompt(&game, &ask(Selection::new(Count::Exactly(2))), &mut handles).expect("a prompt").0 else { panic!("cards") };
        assert!(two.verdict.complete_alone.is_empty(), "no one card answers a choice of two");
        // "As many as can be found" says the number that can be: two wanted, one findable.
        let PromptPayload::ChooseCardsPromptMessage(found) = prompt(&game, &ask(Selection::listed(Count::AsManyAsCan(2), &hand, hand[..1].to_vec())), &mut handles).expect("a prompt").0 else { panic!("cards") };
        assert_eq!((found.count, found.count_kind), (1, CountKindView::AsManyAsCan));
    }

    /// "Any number of target creature cards with total mana value X or less" reaches the client as the limit and each
    /// candidate's mana value, index for index; the default answer takes the first candidates that fit together.
    #[test]
    fn targets_within_a_total_mana_value_send_the_limit_and_each_candidates_value() {
        let (game, hand) = game_with(&["Llanowar Elves", "Ornithopter", "Guerrilla Gorilla"]);
        let seat = game.state.seats().next().expect("a seat");
        let ask = |total: Option<TotalAtMost>| {
            let request = Request {
                id: 1,
                player: seat,
                kind: RequestKind::ChooseTarget {
                    source: hand[0],
                    purpose: Purpose::Targets,
                    candidates: hand[..3].iter().map(|&c| Target::Object(c)).collect(),
                    optional: false,
                    least: 0,
                    count: 3,
                    any_number: true,
                    label: "",
                    slot_player: None,
                    total,
                },
                may_cancel: false,
            };
            match prompt(&game, &request, &mut HandleSpace::new()).expect("a prompt").0 {
                PromptPayload::ChooseTargetPromptMessage(p) => p,
                other => panic!("{other:?}"),
            }
        };
        let limited = ask(Some(TotalAtMost { values: vec![1, 0, 2], most: 1 }));
        assert_eq!((limited.maximum_total_mana_value, limited.mana_values.clone()), (Some(1), vec![1, 0, 2]));
        let AnswerPayload::TargetsChosenAnswer(default) = PromptPayload::ChooseTargetPromptMessage(limited.clone()).default_answer() else {
            panic!("a targets answer")
        };
        // Llanowar Elves (1) and Ornithopter (0) fit within 1; Guerrilla Gorilla (2) would not.
        assert_eq!(default.targets.len(), 2);
        let free = ask(None);
        assert_eq!((free.maximum_total_mana_value, free.mana_values), (None, Vec::new()));
    }

    /// A choice between the words a card's text offers (CR 607.2f) is captioned with every word, in the card's words and
    /// under its name; the same labels offered as anything else are not.
    #[test]
    fn a_choice_of_a_cards_words_is_captioned_with_them() {
        let (game, hand) = game_with(&["Gollum, Riddle Master"]);
        let seat = game.state.seats().next().expect("a seat");
        let card = game.obj(hand[0]).card;
        let ask = |modes: Vec<Label>| {
            let request = Request {
                id: 1,
                player: seat,
                kind: RequestKind::ChooseModes { source: hand[0], modes, eligible: vec![0, 1], count: 1, at_most: 0, affordable: Vec::new(), additional_cost: false, repeat: false },
                may_cancel: false,
            };
            match prompt(&game, &request, &mut HandleSpace::new()).expect("a prompt").0 {
                PromptPayload::ChooseModesPromptMessage(p) => (p.heading, p.modes),
                other => panic!("{other:?}"),
            }
        };
        let (heading, modes) = ask(vec![Label::Option(card, 0), Label::Option(card, 1)]);
        assert_eq!(heading, "Gollum, Riddle Master: choose Odd or Even");
        assert_eq!(modes, vec!["Odd".to_string(), "Even".to_string()]);
        assert_eq!(ask(vec![Label::Card(card, 0), Label::Card(card, 1)]).0, "");
    }

    #[test]
    fn casts_during_resolution_are_asked_in_the_words_of_what_they_cost() {
        assert_eq!(may_verb(Purpose::CastFree), ("CastDuringResolution".to_string(), 1));
        assert_eq!(may_verb(Purpose::Cast), ("CastDuringResolution".to_string(), 0));
        assert_eq!(may_verb(Purpose::CastCopyFree).0, "CastCopyWithoutPaying");
        assert_eq!(may_verb(Purpose::CastForMadness).0, "CastForMadnessCost");
        assert_eq!(may_verb(Purpose::CastTransformedFree).0, "CastTransformed");
        assert_eq!(may_verb(Purpose::CastOrBottom).0, "CastWithoutPaying");
        assert_eq!(may_verb(Purpose::CastFoundOrHand).0, "CastFoundOrPutIntoHand");
    }

    #[test]
    fn the_faces_of_a_card_are_named_with_their_costs_and_say_so() {
        let (game, _) = game_with(&["Bonecrusher Giant"]);
        let giant = card("Bonecrusher Giant").expect("the card");
        assert_eq!(face_text(&game, giant), "Bonecrusher Giant {2}{R}");

        let (detail, costs) = modes_detail(&game, &[Label::Face(giant), Label::Face(giant)], true);
        assert!(detail.contains("Only the face you choose"), "{detail}");
        assert!(!costs);
    }

    /// Miracle's questions name the cost (CR 702.94a), and a choice between miracle abilities names each one's cost and
    /// whose it is: the card's own, or the permanent's that gave it (CR 607.5), whose cost is not on the card's face. A
    /// Room's granted cost is read off each door (CR 709.3a).
    #[test]
    fn miracle_questions_name_the_cost_and_who_gave_it() {
        let (game, _) = game_with(&[]);
        let fanatic = card("Metamorphosis Fanatic").expect("the card");
        let hop = card("Hop to It").expect("the card");
        let room = card("Unholy Annex // Ritual Chamber").expect("the card");
        let lorehold = card("Lorehold, the Historian").expect("the card");
        let aminatou = card("Aminatou, Veil Piercer").expect("the card");
        let own = MiracleOf { card: fanatic, granter: fanatic, index: MiracleOf::OWN };
        let given = MiracleOf { card: hop, granter: lorehold, index: 0 };

        let reveal = miracle_prompt(&game, 1, ViewHandle(7), Purpose::RevealForMiracle(Some(own)));
        assert_eq!((reveal.verb.as_str(), reveal.question.as_str()), ("RevealForMiracle", "Reveal {source} for its miracle {1}{B}?"));
        assert!(reveal.no_default, "revealing is never the default");
        let reveal = miracle_prompt(&game, 1, ViewHandle(7), Purpose::RevealForMiracle(Some(given)));
        assert!(reveal.detail.starts_with("Lorehold, the Historian gives it miracle {2}"), "{}", reveal.detail);
        let several = miracle_prompt(&game, 1, ViewHandle(7), Purpose::RevealForMiracle(None));
        assert_eq!(several.question, "Reveal {source} for one of its miracle abilities?");

        assert_eq!(miracle_label(&game, own), "Miracle {1}{B}, its own");
        assert_eq!(miracle_label(&game, given), "Miracle {2}, from Lorehold, the Historian");
        assert_eq!(miracle_label(&game, MiracleOf { card: hop, granter: aminatou, index: 0 }), "Miracle {W}, from Aminatou, Veil Piercer");
        assert_eq!(
            miracle_label(&game, MiracleOf { card: room, granter: aminatou, index: 0 }),
            "Miracle Unholy Annex {B} or Ritual Chamber {B}{B}, from Aminatou, Veil Piercer"
        );
        let meathook = card("The Meathook Massacre").expect("the card");
        assert_eq!(miracle_label(&game, MiracleOf { card: meathook, granter: aminatou, index: 0 }), "Miracle {X}{B}{B} (X less 4), from Aminatou, Veil Piercer");
        assert_eq!(cost_words(None), "(unpayable: it has no mana cost)");

        let cast = miracle_prompt(&game, 1, ViewHandle(7), Purpose::CastForMiracleCost(given));
        assert_eq!((cast.verb.as_str(), cast.question.as_str()), ("CastForMiracleCost", "Cast {source} for its miracle {2}?"));
        assert!(cast.detail.starts_with("You pay its miracle {2} instead of its mana cost"), "{}", cast.detail);
        assert!(!cast.no_default);
    }

    /// A replacement's source is named with the prompt (KI-34): one that has left - Ral Zarek's skip, Ral in the graveyard -
    /// is named to a seat that holds no handle to it, by its last-known name; one in a hand only to the hand's player.
    #[test]
    fn a_replacements_source_is_named_with_the_prompt_even_without_a_handle() {
        let (mut game, hand) = game_with(&["Llanowar Elves"]);
        let mut seats = game.state.seats();
        let (me, them) = (seats.next().expect("a seat"), seats.next().expect("a seat"));
        let elves = hand[0];
        let ask = |game: &Game, player: PlayerId| {
            let request = Request {
                id: 1,
                player,
                kind: RequestKind::ChooseReplacement {
                    subject: None,
                    life: None,
                    sources: vec![elves],
                    hints: vec!["you skip this turn"],
                    damage: None,
                    turn: true,
                    counters: Vec::new(),
                    draw: false,
                },
                may_cancel: false,
            };
            match prompt(game, &request, &mut HandleSpace::new()).expect("a prompt").0 {
                PromptPayload::ChooseReplacementPromptMessage(p) => p,
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(ask(&game, me).source_names, vec!["Llanowar Elves".to_string()]);
        assert_eq!(ask(&game, them).source_names, vec![String::new()], "a card in another's hand is not named");

        // A card that left a hand (discarded, CR 400.7) is a new object; its old record stays in the zone it left, as the
        // engine leaves it (`ops::move_together`), and that zone is still hidden from the other seat.
        let new = ObjectId(game.state.objects.len() as u32 - 1);
        game.state.objects[elves.index()].became = Some(new);
        assert_eq!(ask(&game, them).source_names, vec![String::new()], "a card that left another's hand is not named");

        // A permanent that left the battlefield - Ral Zarek put into the graveyard - stays recorded there: no handle in a
        // fresh view, but its last-known name.
        game.state.objects[elves.index()].zone = Zone::BATTLEFIELD;
        let asked = ask(&game, them);
        assert!(asked.sources[0].is_none(), "a gone source has no handle in a fresh view");
        assert_eq!(asked.source_names, vec!["Llanowar Elves".to_string()]);
    }
}

/// What a choice of a subtype says it is for: "Choose a creature type" for one an effect names from now on (the heading the
/// bots read it by), and for a word of a text change (CR 612.1), after the spell's name, which word it is - the one replaced, or the one replacing it -
/// and its kind, a creature type or a basic land type.
/// The caption of a choice between the words a card's text offers (CR 607.2f), in its words with the card's name: "Frostcliff
/// Siege: choose Jeskai or Temur", "Mirrodin Besieged: choose Mirran or Phyrexian". The name is the card whose text it is,
/// which a permanent entering as a copy of it is not yet (CR 707.5). None when the options are anything else.
fn options_heading(game: &Game, modes: &[Label]) -> Option<String> {
    let Some(&Label::Option(card, _)) = modes.first() else { return None };
    if !modes.iter().all(|l| matches!(l, Label::Option(c, _) if *c == card)) {
        return None;
    }
    let words: Vec<&str> = modes.iter().map(label_text).collect();
    let listed = match words.split_last() {
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => return None,
    };
    Some(format!("{}: choose {listed}", game.cards.get(card).name))
}

fn subtype_heading(source: &str, choices: &[Subtype], purpose: WordUse) -> String {
    let kind = if choices.first().is_some_and(|s| s.is_creature_type()) { "creature type" } else { "basic land type" };
    match purpose {
        WordUse::Chosen => "Choose a creature type".into(),
        WordUse::Replaced => format!("{source}: choose the {kind} to replace"),
        WordUse::Replacing => format!("{source}: choose the {kind} that replaces it"),
    }
}

/// A count or mana value as the protocol's `int`: exactly, or past its range the most it holds. A count of things that
/// happen is bounded between frames by the event cap, 2^21 (`MAX_GAME_EVENTS`; `docs/rules-exceptions.md`, RX-1), and within
/// one frame only by memory; a mana value is made of a cost's symbols and its X. The bound is there so that nothing sent
/// wraps negative.
pub(crate) fn wire(n: u32) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod wire_tests {
    /// A count past the protocol's `int` is sent as the most it holds, never wrapped to a negative.
    #[test]
    fn a_count_past_the_protocols_int_stops_at_its_bound() {
        assert_eq!(super::wire(300), 300);
        assert_eq!(super::wire(i32::MAX as u32), i32::MAX);
        assert_eq!(super::wire(u32::MAX), i32::MAX);
    }
}
