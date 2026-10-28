//! A seat's `PlayerView` from the native engine's state: what that seat may see and nothing else.
//!
//! Visibility (`cantrip_engine::visibility`, shared with the AI's observations) follows the C# projector (`Cantrip.Projection.PlayerViewProjector`): the battlefield, the stack, the
//! graveyards, exile and the command zone to everyone, a hand to its owner, a library to nobody (CR 401.2). Cards a
//! prompt or a reveal is showing right now cross to the seat they are shown to (`extra`), as the C# `alsoReveal` does:
//! they are objects of the view with handles, though the zone they sit in stays closed.
//!
//! Characteristics are computed (`Game::chars`), never printed: power, toughness, types, colors and subtypes come out
//! of the layers, so an anthem shows.
//!
//! # Left at the default, because the engine has no notion of it
//!
//! - `ObjectView::fingerprint` and `spent_lines`.
//! - `known_by` and `ZoneView::known`: the AI's `Knowledge` (cantrip-ai) folds who has been shown which hidden card
//!   from the history, but the host does not use it yet, so only a reveal of this step crosses (as an object).
//! - `SpellTargetView::text` is empty and `still_legal` always true.
//! - `PlayerSummaryView::counters`: not filled.

use std::collections::HashSet;

use cantrip_engine::*;
pub use cantrip_engine::visibility::*;

use crate::handles::HandleSpace;
use crate::protocol::*;

/// Seat number of a player: one-based.
pub fn seat(player: PlayerId) -> i32 {
    i32::from(player.0) + 1
}

/// The player at a seat number, or none for zero.
pub fn player_at(seat: i32) -> Option<PlayerId> {
    (seat > 0).then(|| PlayerId((seat - 1) as u8))
}

pub fn zone_kind_view(kind: ZoneKind) -> ZoneKindView {
    match kind {
        ZoneKind::Library => ZoneKindView::Library,
        ZoneKind::Hand => ZoneKindView::Hand,
        ZoneKind::Graveyard => ZoneKindView::Graveyard,
        ZoneKind::Exile => ZoneKindView::Exile,
        ZoneKind::Battlefield => ZoneKindView::Battlefield,
        ZoneKind::Stack => ZoneKindView::Stack,
        ZoneKind::Command => ZoneKindView::Command,
        ZoneKind::Sideboard => ZoneKindView::Sideboard,
        // Where a token that left, or an object that never was, would be: the closest thing the wire has.
        ZoneKind::Gone => ZoneKindView::Exile,
        // A phased-out permanent is still a permanent (CR 702.26d), drawn on the battlefield as phased out
        // (`ObjectView::phased_out`).
        ZoneKind::PhasedOut => ZoneKindView::Battlefield,
    }
}

/// Seat owning a zone, or zero for a shared one.
pub fn zone_owner(zone: Zone) -> i32 {
    match zone.kind() {
        ZoneKind::Library | ZoneKind::Hand | ZoneKind::Graveyard | ZoneKind::Sideboard => seat(zone.player),
        _ => 0,
    }
}

pub fn step_view(step: Step) -> TurnStepView {
    match step {
        Step::Untap => TurnStepView::Untap,
        Step::Upkeep => TurnStepView::Upkeep,
        Step::Draw => TurnStepView::Draw,
        Step::PrecombatMain => TurnStepView::PrecombatMain,
        Step::BeginCombat => TurnStepView::BeginCombat,
        Step::DeclareAttackers => TurnStepView::DeclareAttackers,
        Step::DeclareBlockers => TurnStepView::DeclareBlockers,
        Step::CombatDamage => TurnStepView::CombatDamage,
        Step::EndOfCombat => TurnStepView::EndOfCombat,
        Step::PostcombatMain => TurnStepView::PostcombatMain,
        Step::End => TurnStepView::End,
        Step::Cleanup => TurnStepView::Cleanup,
    }
}

pub fn color_view(color: Color) -> ColorView {
    match color {
        Color::White => ColorView::White,
        Color::Blue => ColorView::Blue,
        Color::Black => ColorView::Black,
        Color::Red => ColorView::Red,
        Color::Green => ColorView::Green,
    }
}

pub fn color_of(view: ColorView) -> Color {
    match view {
        ColorView::White => Color::White,
        ColorView::Blue => Color::Blue,
        ColorView::Black => Color::Black,
        ColorView::Red => Color::Red,
        ColorView::Green => Color::Green,
    }
}

/// What a restriction on mana is, as the wire says it (CR 106.6): what the mana may be spent on, what it does to what it
/// pays for, the subtypes the spell must have one of and the least mana value it may have. Every restriction has its
/// own, so `restriction_of` reads it back.
pub fn restriction_view(restriction: ManaRestriction) -> (ManaSpendFilterView, ManaSpendEffectView, Vec<String>, i32) {
    let mut subtypes = Vec::new();
    let mut mana_value = 0;
    let filter = match restriction {
        ManaRestriction::ManaValue(m) => {
            mana_value = i32::from(m.at_least);
            if m.or_x { ManaSpendFilterView::ManaValueOrXSpells } else { ManaSpendFilterView::ManaValueSpells }
        }
        r if r == ManaRestriction::NONCREATURE_SPELLS => ManaSpendFilterView::NoncreatureSpells,
        ManaRestriction::Subtypes(s) => {
            subtypes = s.subtypes.iter().flatten().map(|t| t.name().to_string()).collect();
            match (s.creature, s.abilities) {
                (true, Some(AbilitiesOf::Sources)) => ManaSpendFilterView::SubtypeCreatureSpellsOrAbilities,
                (false, Some(AbilitiesOf::Sources)) => ManaSpendFilterView::SubtypeSpellsOrAbilities,
                (false, Some(AbilitiesOf::Permanents)) => ManaSpendFilterView::SubtypeSpellsOrAbilitiesOfPermanents,
                (true, None) => ManaSpendFilterView::SubtypeCreatureSpells,
                (false, None) => ManaSpendFilterView::SubtypeSpells,
                (true, Some(AbilitiesOf::Permanents)) => panic!("a mana restriction the wire has no word for: {}", restriction.name()),
            }
        }
        ManaRestriction::NotNonartifactSpells => ManaSpendFilterView::NotNonartifactSpells,
        ManaRestriction::GainsHaste => ManaSpendFilterView::None,
        ManaRestriction::LegendaryUncounterable => ManaSpendFilterView::LegendarySpells,
        r if r == ManaRestriction::ARTIFACT_SPELLS => ManaSpendFilterView::ArtifactSpells,
        r if r == ManaRestriction::INSTANT_OR_SORCERY_SPELLS => ManaSpendFilterView::InstantOrSorcerySpells,
        r if r == ManaRestriction::PLANESWALKER_SPELLS => ManaSpendFilterView::PlaneswalkerSpells,
        r if r == ManaRestriction::ABILITIES => ManaSpendFilterView::ActivatedAbilities,
        r if r == ManaRestriction::LAND_ABILITIES => ManaSpendFilterView::LandAbilities,
        r if r == ManaRestriction::ARTIFACT_ABILITIES => ManaSpendFilterView::ArtifactAbilities,
        r if r == ManaRestriction::CREATURE_SPELLS => ManaSpendFilterView::CreatureSpells,
        r if r == ManaRestriction::CREATURE_ABILITIES => ManaSpendFilterView::CreatureAbilities,
        r if r == ManaRestriction::SPELLS_FROM_EXILE => ManaSpendFilterView::SpellsFromExile,
        r if r == ManaRestriction::NOT_SPELLS_FROM_HAND => ManaSpendFilterView::NotSpellsFromHand,
        r if r == ManaRestriction::ABILITIES_OF_ARTIFACTS => ManaSpendFilterView::AbilitiesOfArtifacts,
        r if r == ManaRestriction::CREATURE_SPELLS_OR_ABILITIES_OF_CREATURES => ManaSpendFilterView::CreatureSpellsOrAbilitiesOfCreatures,
        r if r == ManaRestriction::CREATURE_SPELLS_OR_ABILITIES => ManaSpendFilterView::CreatureSpellsOrAbilities,
        r if r == ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES_OF_ARTIFACTS => ManaSpendFilterView::ArtifactSpellsOrAbilitiesOfArtifacts,
        r => panic!("a mana restriction the wire has no word for: {}", r.name()),
    };
    let effect = match restriction.effects() {
        MANA_GIVES_HASTE => ManaSpendEffectView::GainsHaste,
        MANA_MAKES_UNCOUNTERABLE => ManaSpendEffectView::CantBeCountered,
        _ => ManaSpendEffectView::None,
    };
    (filter, effect, subtypes, mana_value)
}

/// The restriction a filter, an effect and subtypes from the wire name; none for unrestricted mana.
pub fn restriction_of(filter: ManaSpendFilterView, effect: ManaSpendEffectView, subtypes: &[String], mana_value: i32) -> Option<ManaRestriction> {
    if (filter, effect) == (ManaSpendFilterView::None, ManaSpendEffectView::None) {
        return None;
    }
    if matches!(filter, ManaSpendFilterView::ManaValueSpells | ManaSpendFilterView::ManaValueOrXSpells) {
        let at_least = u8::try_from(mana_value).ok()?;
        return Some(ManaRestriction::ManaValue(ManaValueSpells { at_least, or_x: filter == ManaSpendFilterView::ManaValueOrXSpells }));
    }
    if matches!(
        filter,
        ManaSpendFilterView::SubtypeSpells
            | ManaSpendFilterView::SubtypeCreatureSpells
            | ManaSpendFilterView::SubtypeSpellsOrAbilities
            | ManaSpendFilterView::SubtypeCreatureSpellsOrAbilities
            | ManaSpendFilterView::SubtypeSpellsOrAbilitiesOfPermanents
    ) {
        if subtypes.len() > 2 {
            return None;
        }
        let mut named = [None, None];
        for (slot, name) in named.iter_mut().zip(subtypes) {
            *slot = Some(Subtype::named(name)?);
        }
        return Some(ManaRestriction::Subtypes(SubtypeSpells {
            subtypes: named,
            chosen: false,
            creature: matches!(filter, ManaSpendFilterView::SubtypeCreatureSpells | ManaSpendFilterView::SubtypeCreatureSpellsOrAbilities),
            uncounterable: effect == ManaSpendEffectView::CantBeCountered,
            abilities: match filter {
                ManaSpendFilterView::SubtypeSpellsOrAbilities | ManaSpendFilterView::SubtypeCreatureSpellsOrAbilities => Some(AbilitiesOf::Sources),
                ManaSpendFilterView::SubtypeSpellsOrAbilitiesOfPermanents => Some(AbilitiesOf::Permanents),
                _ => None,
            },
        }));
    }
    ManaRestriction::NAMED.into_iter().map(|(r, _)| r).find(|&r| restriction_view(r) == (filter, effect, Vec::new(), 0))
}

pub fn mana_view(mana: Mana) -> ManaTypeView {
    match mana {
        Mana::W => ManaTypeView::White,
        Mana::U => ManaTypeView::Blue,
        Mana::B => ManaTypeView::Black,
        Mana::R => ManaTypeView::Red,
        Mana::G => ManaTypeView::Green,
        Mana::C => ManaTypeView::Colorless,
    }
}

pub fn mana_of(view: ManaTypeView) -> Mana {
    match view {
        ManaTypeView::White => Mana::W,
        ManaTypeView::Blue => Mana::U,
        ManaTypeView::Black => Mana::B,
        ManaTypeView::Red => Mana::R,
        ManaTypeView::Green => Mana::G,
        ManaTypeView::Colorless => Mana::C,
    }
}

pub fn colors_view(set: ColorSet) -> Vec<ColorView> {
    set.iter().map(color_view).collect()
}

/// The order a type line prints card types in.
const TYPE_ORDER: [(CardType, CardTypeView); 9] = [
    (CardType::Kindred, CardTypeView::Kindred),
    (CardType::Artifact, CardTypeView::Artifact),
    (CardType::Enchantment, CardTypeView::Enchantment),
    (CardType::Land, CardTypeView::Land),
    (CardType::Planeswalker, CardTypeView::Planeswalker),
    (CardType::Battle, CardTypeView::Battle),
    (CardType::Creature, CardTypeView::Creature),
    (CardType::Instant, CardTypeView::Instant),
    (CardType::Sorcery, CardTypeView::Sorcery),
];

pub fn types_view(types: TypeSet) -> Vec<CardTypeView> {
    TYPE_ORDER.iter().filter(|(t, _)| types.has(*t)).map(|(_, v)| *v).collect()
}

pub fn supertypes_view(s: Supertypes) -> Vec<SupertypeView> {
    let mut out = Vec::new();
    if s.legendary {
        out.push(SupertypeView::Legendary);
    }
    if s.basic {
        out.push(SupertypeView::Basic);
    }
    if s.snow {
        out.push(SupertypeView::Snow);
    }
    out
}

pub fn subtypes_view(set: SubtypeSet) -> Vec<String> {
    set.iter().map(|t| t.name().to_string()).collect()
}

/// "Hexproof from [quality]" as a card's text words it (CR 702.11d, 702.11f-g): "Hexproof from monocolored", "Hexproof from
/// white and from black", "Hexproof from artifacts, creatures, and enchantments", "Hexproof from each color".
pub fn hexproof_from_text(qualities: HexproofFrom) -> String {
    let mut words: Vec<String> = Vec::new();
    if qualities.colors() == ColorSet(0x1f) {
        words.push("each color".to_string());
    } else {
        words.extend(qualities.colors().iter().map(|c| c.name().to_lowercase()));
    }
    if qualities.has(HexproofFrom::MONOCOLORED) {
        words.push("monocolored".to_string());
    }
    if qualities.has(HexproofFrom::MULTICOLORED) {
        words.push("multicolored".to_string());
    }
    words.extend(qualities.types().iter().map(|t| format!("{}s", t.name().to_lowercase())));
    match words.len() {
        0 => String::new(),
        1 => format!("Hexproof from {}", words[0]),
        2 => format!("Hexproof from {} and from {}", words[0], words[1]),
        n => format!("Hexproof from {}, and {}", words[..n - 1].join(", "), words[n - 1]),
    }
}

/// The keywords as a card's text words them, for the ones an effect gave an object or took from it.
const KEYWORD_WORDS: [(Keyword, &str); 51] = [
    (Keyword::Flying, "Flying"),
    (Keyword::Haste, "Haste"),
    (Keyword::Vigilance, "Vigilance"),
    (Keyword::Trample, "Trample"),
    (Keyword::FirstStrike, "First strike"),
    (Keyword::DoubleStrike, "Double strike"),
    (Keyword::Deathtouch, "Deathtouch"),
    (Keyword::Lifelink, "Lifelink"),
    (Keyword::Reach, "Reach"),
    (Keyword::Defender, "Defender"),
    (Keyword::Flash, "Flash"),
    (Keyword::Hexproof, "Hexproof"),
    (Keyword::Indestructible, "Indestructible"),
    (Keyword::Menace, "Menace"),
    (Keyword::Infect, "Infect"),
    (Keyword::Shadow, "Shadow"),
    (Keyword::Shroud, "Shroud"),
    (Keyword::CantBeCountered, "This spell can't be countered."),
    (Keyword::CantBlock, "This creature can't block."),
    (Keyword::CanBlockOnlyCreaturesWithFlying, "This creature can block only creatures with flying."),
    (Keyword::DoesNotUntap, "This permanent doesn't untap during your untap step."),
    (Keyword::CrewsAsThoughPowerTwoGreater, "This creature crews Vehicles as though its power were 2 greater."),
    (Keyword::CantBeBlocked, "This creature can't be blocked."),
    (Keyword::CantSpendManaToCast, "You can't spend mana to cast this spell."),
    (Keyword::CastFromYourGraveyard, "You may cast this card from your graveyard."),
    (Keyword::Retrace, "Retrace"),
    (Keyword::Daybound, "Daybound"),
    (Keyword::Nightbound, "Nightbound"),
    (Keyword::SplitSecond, "Split second"),
    (Keyword::CantBeAttacked, "This permanent can't be attacked."),
    (Keyword::CantBeBlockedByPowerTwoOrLess, "This creature can't be blocked by creatures with power 2 or less."),
    (Keyword::CantBeBlockedExceptByThreeOrMore, "This creature can't be blocked except by three or more creatures."),
    (Keyword::Changeling, "Changeling"),
    (Keyword::Decayed, "Decayed"),
    (Keyword::CantBeBlockedByMoreThanOne, "This creature can't be blocked by more than one creature."),
    (Keyword::Islandwalk, "Islandwalk"),
    (Keyword::Plainswalk, "Plainswalk"),
    (Keyword::Swampwalk, "Swampwalk"),
    (Keyword::Mountainwalk, "Mountainwalk"),
    (Keyword::Forestwalk, "Forestwalk"),
    (Keyword::Banding, "Banding"),
    (Keyword::SaddlesAsThoughPowerTwoGreater, "This creature saddles Mounts as though its power were 2 greater."),
    (Keyword::CantBeSacrificed, "This permanent can't be sacrificed."),
    (Keyword::Horsemanship, "Horsemanship"),
    (Keyword::Fear, "Fear"),
    (Keyword::Intimidate, "Intimidate"),
    (Keyword::Skulk, "Skulk"),
    (Keyword::NonbasicLandwalk, "Nonbasic landwalk"),
    (Keyword::LegendaryLandwalk, "Legendary landwalk"),
    (Keyword::Wither, "Wither"),
    (Keyword::Phasing, "Phasing"),
];

/// A rules line without its parenthesized reminder text, trimmed.
fn without_reminder(line: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for ch in line.chars() {
        match ch {
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// A mana cost in oracle notation; empty for no mana cost at all (CR 202.1b).
pub fn cost_text(def: &CardDef) -> String {
    if def.has_cost { def.cost.to_string() } else { String::new() }
}

/// One printed card as a card to be read: its name, cost, type line, text and printed power and toughness, and its
/// colors from its cost and color indicator (CR 202.2). No handle, no state.
pub fn printed(def: &CardDef) -> ObjectView {
    ObjectView {
        handle: ViewHandle::NONE,
        known: true,
        card_id: def.name.to_string(),
        name: def.name.to_string(),
        mana_cost: cost_text(def),
        mana_value: if def.has_cost { def.cost.mana_value() as i32 } else { 0 },
        oracle_text: def.text.to_string(),
        supertypes: supertypes_view(def.supertypes),
        card_types: types_view(def.types),
        subtypes: subtypes_view(def.subtypes),
        colors: colors_view(def.colors()),
        power: def.power.map(i32::from),
        toughness: def.toughness.map(i32::from),
        produces_mana: def.abilities.iter().any(|a| matches!(a, AbilityDef::Activated(d) if d.is_mana_ability())),
        starting_counter: def.loyalty.map(i32::from),
        station_pt: cantrip_oracle::station_pt(def.name).unwrap_or_default().to_string(),
        other_face: def.back.map(|back| Box::new(printed(back))),
        halves: def.split.map_or_else(Vec::new, |(left, right)| vec![printed(left), printed(right)]),
        inset_spell: inset_spell(def),
        prototype: def.prototype.map_or_else(String::new, |p| p.text.to_string()),
        prototyped: def.prototyped,
        ..Default::default()
    }
}

/// The Adventure or Omen printed in a card's inset frame (CR 715.2, 720.2), as printed, for a player to read beside the
/// card: it is part of the card in every zone, though the card has only its normal characteristics there (CR 715.4, 720.4).
fn inset_spell(def: &CardDef) -> Option<Box<ObjectView>> {
    def.inset_faces().next().map(|(_, face)| Box::new(printed(face)))
}

fn target_view(game: &Game, handles: &mut HandleSpace, target: Target) -> Option<TargetMessage> {
    match target {
        Target::Player(p) => Some(TargetMessage::player(seat(p))),
        Target::Object(id) => Some(TargetMessage::object(handles.handle(id))),
        Target::Ability(serial) => {
            let _ = game;
            Some(TargetMessage::object(handles.handle(ability_key(serial))))
        }
        Target::None => None,
    }
}

pub struct Projector<'a> {
    pub game: &'a Game,
    pub viewer: PlayerId,
    pub names: &'a [String],
    /// Seats that conceded: they read as such rather than as lost.
    pub conceded: &'a [bool],
    /// Objects whose face is shown to the viewer right now: revealed (CR 701.20a) or looked at (CR 701.20e). A face-down
    /// permanent among them is seen as what it really is while it is shown, though it stays face down (a manifested or cloaked
    /// instant an effect would turn up, CR 701.40g, 701.58g); once it is not, the viewer may look at it only as `knows_face`
    /// allows (CR 708.5).
    pub faces: &'a HashSet<ObjectId>,
}

/// `viewer`'s view of the game. `extra` are objects shown to the viewer right now, `faces` those of them whose face is
/// shown (`Projector::faces`), and `mentioned` objects whose handles must survive this projection because the events going
/// out beside it name them.
pub fn player_view(
    game: &Game,
    viewer: PlayerId,
    handles: &mut HandleSpace,
    names: &[String],
    conceded: &[bool],
    extra: &HashSet<ObjectId>,
    faces: &HashSet<ObjectId>,
    mentioned: &HashSet<ObjectId>,
) -> PlayerView {
    let projector = Projector { game, viewer, names, conceded, faces };
    projector.build(handles, extra, mentioned)
}

impl<'a> Projector<'a> {
    fn build(&self, handles: &mut HandleSpace, extra: &HashSet<ObjectId>, mentioned: &HashSet<ObjectId>) -> PlayerView {
        let game = self.game;
        let s = &game.state;
        let mut visible = observable(game, self.viewer, extra);
        // A face-down permanent or spell revealed as its owner left the game (CR 708.9) has ceased to exist, in no zone, but it
        // is shown right now all the same, so that what was revealed can be named.
        visible.extend(extra.iter().copied().filter(|&id| {
            !is_ability_key(id) && id.index() < s.objects.len() && s.obj(id).zone.is(ZoneKind::Gone) && s.obj(id).became.is_none()
        }));
        // Before anything is minted: whatever this viewer can no longer see loses its handle, so if it ever comes back
        // it comes back unrecognisable.
        handles.retain(|o| visible.contains(&o) || mentioned.contains(&o));

        let pending = game.pending();
        let mut view = PlayerView {
            viewer: seat(self.viewer),
            turn: s.turn.number as i32,
            step: Some(step_view(s.turn.step())),
            active_player: seat(s.turn.active),
            priority_player: match pending {
                Some(r) if matches!(r.kind, RequestKind::Priority { .. }) => seat(r.player),
                _ => 0,
            },
            day_night: DayNightView::Neither,
            ..Default::default()
        };
        match s.outcome {
            Outcome::InProgress => {}
            Outcome::Draw => view.outcome = GameOutcomeView::Draw,
            Outcome::Won(p) => {
                view.outcome = GameOutcomeView::TeamWon;
                view.winning_team = seat(p);
            }
        }

        for p in s.seats() {
            view.players.push(self.summarise(p, handles));
        }

        // In id order so the wire bytes depend on the position and not on map enumeration order.
        let mut ordered: Vec<ObjectId> = visible.iter().copied().collect();
        ordered.sort();
        let entries = stack_entries(s);
        for id in ordered {
            let handle = handles.handle(id);
            if is_ability_key(id) {
                if let Some(StackEntry::Ability { item }) = entries.iter().find(|e| e.key() == id) {
                    let described = self.describe_ability(item, handle, handles);
                    view.objects.push(described);
                }
                continue;
            }
            let item = entries.iter().find_map(|e| match e {
                StackEntry::Spell { id: s, item } if *s == id => *item,
                _ => None,
            });
            let described = self.describe(id, handle, item, handles);
            view.objects.push(described);
        }
        view.objects.sort_by_key(|o| o.handle);

        for p in s.seats() {
            view.zones.push(self.zone_view(ZoneKind::Library, p, handles, &visible));
            view.zones.push(self.zone_view(ZoneKind::Hand, p, handles, &visible));
            view.zones.push(self.zone_view(ZoneKind::Graveyard, p, handles, &visible));
            view.zones.push(self.zone_view(ZoneKind::Sideboard, p, handles, &visible));
        }
        for kind in [ZoneKind::Battlefield, ZoneKind::Stack, ZoneKind::Exile, ZoneKind::Command] {
            view.zones.push(self.zone_view(kind, PlayerId(0), handles, &visible));
        }
        view.combat = self.combat(handles);
        view
    }

    /// A player as the subject of a sentence: "You" for the viewer, else their name.
    fn player_name(&self, p: PlayerId) -> String {
        if p == self.viewer { "You".to_string() } else { self.names.get(p.index()).cloned().unwrap_or_else(|| format!("Player {}", p.0 + 1)) }
    }

    fn summarise(&self, p: PlayerId, handles: &mut HandleSpace) -> PlayerSummaryView {
        let game = self.game;
        let s = &game.state;
        let player = s.player(p);
        let mut summary = PlayerSummaryView {
            seat: seat(p),
            name: self.names.get(p.index()).cloned().unwrap_or_default(),
            team: seat(p),
            life: player.life,
            status: if self.conceded.get(p.index()).copied().unwrap_or(false) {
                PlayerStatusView::Conceded
            } else if player.lost {
                PlayerStatusView::Lost
            } else {
                PlayerStatusView::Active
            },
            poison_counters: i32::from(player.poison()),
            hand_count: s.zone(Zone::hand(p)).len() as i32,
            library_count: s.zone(Zone::library(p)).len() as i32,
            lands_played_this_turn: crate::prompt::wire(player.lands_played),
            is_monarch: player.monarch,
            has_initiative: player.initiative,
            has_citys_blessing: player.citys_blessing,
            has_enduring_story: player.enduring_story,
            speed: player.speed.map(i32::from),
            energy: i32::from(player.energy()),
            // Asked of the history: every completion is recorded (CR 309.7).
            dungeons_completed: crate::prompt::wire(count_u32(game.history().select(EventQuery::kind(EventMask::DUNGEON_COMPLETED).player(p)).count())),
            ..Default::default()
        };
        for mana in Mana::ALL {
            summary.mana[mana as usize] = count(player.pool.get(mana));
        }
        // Ordinary mana a rule tells apart by the source that produced it (CR 106.3) is a kind of its own, as restricted mana
        // is: that source's, shown apart from the rest and named by it.
        for source in game.mana_told_apart(p) {
            for e in s.mana_from_of(p).filter(|e| e.restriction.is_none() && e.source == source) {
                for mana in Mana::ALL {
                    let n = e.pool.get(mana);
                    if n > 0 {
                        summary.mana[mana as usize] -= count(n);
                        let handle = handles.handle(source);
                        match summary.restricted_mana.iter_mut().find(|r| r.source == handle && r.r#type == Some(mana_view(mana))) {
                            Some(r) => r.amount += count(n),
                            None => summary.restricted_mana.push(RestrictedManaView {
                                r#type: Some(mana_view(mana)),
                                amount: count(n),
                                source: handle,
                                ..Default::default()
                            }),
                        }
                    }
                }
            }
        }
        for (restriction, pool) in s.restricted_of(p) {
            for mana in Mana::ALL {
                let n = pool.get(mana);
                if n > 0 {
                    let (filter, effect, spend_subtypes, spend_mana_value) = restriction_view(restriction);
                    summary.restricted_mana.push(RestrictedManaView {
                        r#type: Some(mana_view(mana)),
                        amount: count(n),
                        filter,
                        effect,
                        spend_subtypes,
                        spend_mana_value,
                        source: ViewHandle::NONE,
                    });
                }
            }
        }
        // "The next spell you cast this turn can't be countered" (CR 611.2c): an effect about the player, one per activation,
        // that sits on no object.
        let waiting = s.effects.iter().filter(|e| e.controller == p && e.kind == EffectKind::NextSpellCantBeCountered).count();
        for _ in 0..waiting {
            let whose = if p == self.viewer { "you" } else { "they" };
            summary.pending_effects.push(format!("The next spell {whose} cast this turn can't be countered."));
        }
        // "You may play an additional land this turn" (CR 305.2, 611.2c): a permission about the player, one effect for each
        // resolution that gave it, on no object; the lands they add are summed, so the page says how many more may be played.
        let extra_lands: u32 = s.effects.iter().filter(|e| e.controller == p).map(|e| if let EffectKind::ExtraLandPlays(n) = e.kind { n } else { 0 }).sum();
        if extra_lands > 0 {
            let who = if p == self.viewer { "You" } else { "They" };
            let lands = if extra_lands == 1 { "an additional land".to_string() } else { format!("{extra_lands} additional lands") };
            summary.pending_effects.push(format!("{who} may play {lands} this turn."));
        }
        // "Target opponent skips their next X turns" (CR 500.11, 614.10): one effect for each turn still to skip, about the
        // player and on no object.
        let skips = s.effects.iter().filter(|e| e.kind == EffectKind::SkipNextTurn(p)).count();
        if skips > 0 {
            let (who, whose) = if p == self.viewer { ("You", "your") } else { ("They", "their") };
            let turns = if skips == 1 { "turn".to_string() } else { format!("{skips} turns") };
            summary.pending_effects.push(format!("{who} skip {whose} next {turns}."));
        }
        // "You control target player during that player's next turn" (CR 723.1): the control now, and each effect still
        // waiting for the player's next turn, about the player and on no object. The last created is the one that will work
        // (723.1a); each that gives an extra turn after it gives it whichever works.
        if let Some(companion) = revealed_companion(s, p) {
            summary.companion = handles.handle(companion);
        }
        let controller = s.player_controller(p);
        if controller != p {
            summary.controlled_by = seat(controller);
            let (who, whose) = if p == self.viewer { ("you", "your") } else { ("them", "their") };
            let verb = if controller == self.viewer { "control" } else { "controls" };
            summary.pending_effects.push(format!("{} {verb} {who} during {whose} turn.", self.player_name(controller)));
        }
        for e in s.effects.iter() {
            if let EffectKind::ControlNextTurn { player, then_extra_turn } = e.kind
                && player == p
            {
                let (who, whose, they) = if p == self.viewer { ("you", "your", "you take") } else { ("them", "their", "they take") };
                let verb = if e.controller == self.viewer { "control" } else { "controls" };
                let extra = if then_extra_turn { format!(" After that turn, {they} an extra turn.") } else { String::new() };
                summary.pending_effects.push(format!("{} {verb} {who} during {whose} next turn.{extra}", self.player_name(e.controller)));
            }
        }
        // "The 'legend rule' doesn't apply to permanents you control this turn" (CR 704.5j, 611.2c): a rule about the player, on
        // no object.
        if let Some(e) = s.effects.iter().find(|e| e.kind == EffectKind::NoLegendRule(p)) {
            let whose = if p == self.viewer { "you" } else { "they" };
            let until = if e.duration == Duration::EndOfTurn { " this turn" } else { "" };
            summary.pending_effects.push(format!("The \"legend rule\" doesn't apply to permanents {whose} control{until}."));
        }
        // "Artifact spells you cast this turn cost {1} less to cast" (CR 611.2c, 601.2f): a cost change a resolved spell or ability
        // made about the player's spells, on no object, so the page says it beside their life total in the card's own words,
        // named for the card that made it. The other seat is told whose spells it changes.
        for e in s.effects.iter() {
            if let EffectKind::SpellCostChange { card, index, player } = e.kind
                && player == p
            {
                let def = game.cards.get(card);
                let text = def.lasting_cost_changes[index as usize].text;
                summary.pending_effects.push(if p == self.viewer {
                    format!("{}: {text}", def.name)
                } else {
                    format!("{}'s effect on their spells: {text}", def.name)
                });
            }
        }
        // "Whenever [event] this turn" / "when you next [event] this turn" / "until your next turn, whenever [event]" (CR 603.7b):
        // a delayed triggered ability that waits for its event, on no object, so the page says it beside its controller's life
        // total, in the ability's own words
        // and named for the object it came from (the words may say "that creature"). The other seat is told whose it is. So is
        // one that triggers at each such step for the rest of the game (paradigm, CR 702.192a): the card it came from may be
        // anywhere, or nowhere it can be seen; and one waiting for a step of an extra turn (Final Fortune's "at the beginning of
        // that turn's end step, you lose the game"), whose spell is gone.
        for d in s.delayed.iter().filter(|d| {
            d.controller == p
                && matches!(
                    d.when,
                    DelayedWhen::ThisTurn | DelayedWhen::NextThisTurn | DelayedWhen::UntilNextTurnOf(_) | DelayedWhen::EachStep { .. } | DelayedWhen::ExtraTurnStep { .. }
                )
        }) {
            let text = game.cards.ability(d.ability).text();
            let from = game.cards.get(game.chars(d.source).card).name;
            summary.pending_effects.push(if p == self.viewer {
                format!("{from}: {text}")
            } else {
                format!("They have {from}'s delayed ability: {text}")
            });
        }
        // "You may activate loyalty abilities of Jace planeswalkers you control ... any time you could cast an instant"
        // (CR 606.3, 611.2c): a permission about the player, on no object.
        for e in s.effects.iter().filter(|e| e.controller == p) {
            if let EffectKind::InstantLoyalty(kind) = e.kind {
                let they = if p == self.viewer { "you" } else { "they" };
                let walkers = kind.map_or("planeswalkers".to_string(), |t| format!("{} planeswalkers", t.name()));
                let until = if e.duration == Duration::EndOfTurn { " this turn" } else { "" };
                summary.pending_effects.push(format!(
                    "{}{} may activate loyalty abilities of {walkers} {they} control any time {they} could cast an instant{until}.",
                    they[..1].to_uppercase(),
                    &they[1..]
                ));
            }
        }
        // "Your opponents can't cast spells from anywhere other than their hands" until a turn (CR 101.2, 611.2c): a restriction
        // on the player, on no object, so the player it stops is told where they may still cast from.
        for e in s.effects.iter() {
            if let EffectKind::CantCastSpellsFrom(barred, zones) = e.kind
                && barred == p
            {
                summary.pending_effects.push(self.cant_cast_text(p, zones, e.duration));
            }
        }
        // A prevention effect a spell or ability made about the player (CR 615.7: "prevent the next 3 damage that would be dealt
        // to target player"), shown beside the player it shields; one of damage to anyone (Fog) beside its controller's.
        for e in s.effects.iter() {
            let whom = match e.kind {
                EffectKind::Prevention { to: Target::Player(q), .. } if q == p => {
                    if p == self.viewer { "you" } else { "them" }
                }
                // One about an object it names more than as the damage's recipient is on that object (Maze of Ith's).
                EffectKind::Prevention { to: Target::None, card, index, .. }
                    if e.controller == p && self.game.cards.get(card).granted_preventions.get(index as usize).is_some_and(|d| d.of_it.is_none()) =>
                {
                    ""
                }
                _ => continue,
            };
            if let Some(text) = self.prevention_text(e, whom) {
                summary.pending_effects.push(text);
            }
        }
        // A player's own protection, hexproof and unchanging life total (CR 702.16j, 702.11c, 119.7-119.8): resolved effects about
        // the player (Teferi's Protection, The One Ring, Dawn's Truce), with how long they last.
        for e in s.effects.iter() {
            let you = if p == self.viewer { "You" } else { "They" };
            let text = match e.kind {
                EffectKind::PlayerProtectionFromEverything if e.controller == p => format!("{you} have protection from everything"),
                EffectKind::PlayerHexproof if e.controller == p => format!("{you} have hexproof"),
                EffectKind::CantActivatePowerUp => "Power-up abilities can't be activated".to_string(),
                EffectKind::CreaturesAssignDamageByToughness if e.controller == p => {
                    format!("{} creatures with toughness greater than power assign combat damage equal to toughness", if p == self.viewer { "Your" } else { "Their" })
                }
                EffectKind::LifeTotalCantChange(q) if q == p => {
                    format!("{} life total can't change", if p == self.viewer { "Your" } else { "Their" })
                }
                _ => continue,
            };
            summary.pending_effects.push(format!("{text}{}.", self.until_text(e.duration)));
        }
        // "You have shroud" (CR 702.18a) and hexproof (CR 702.11c) from a static ability of a permanent, as it applies now.
        for (keyword, word) in [(Keyword::Shroud, "shroud"), (Keyword::Hexproof, "hexproof")] {
            if game.player_has(p, keyword) {
                summary.pending_effects.push(format!("{} have {word}.", if p == self.viewer { "You" } else { "They" }));
            }
        }
        // Protection from players (CR 702.16k) from a static ability of a permanent, as it applies now: "each of your
        // opponents" when it covers every one of them (Absolute Virtue), else the players by name.
        let opponents: Vec<PlayerId> = s.opponents(p).collect();
        let from: Vec<PlayerId> = opponents.iter().copied().filter(|&o| game.player_protected_by_static_from(p, o)).collect();
        if !from.is_empty() {
            let you = if p == self.viewer { "You" } else { "They" };
            let whom = if from.len() == opponents.len() && from.len() > 1 {
                format!("each of {} opponents", if p == self.viewer { "your" } else { "their" })
            } else {
                let names: Vec<String> =
                    from.iter().map(|o| self.names.get(o.0 as usize).cloned().unwrap_or_else(|| format!("player {}", o.0 + 1))).collect();
                names.join(" and from ")
            };
            summary.pending_effects.push(format!("{you} have protection from {whom}."));
        }
        // "Players can't gain life" (CR 119.7): from a permanent's static ability while it is on the battlefield, or a resolved
        // effect about the player (Screaming Nemesis's "for the rest of the game"), as it applies now.
        // "Can't lose life" (CR 119.8) likewise, from a static ability; both at once are a life total that can't change
        // (Platinum Emperion).
        let you = p == self.viewer;
        match (game.cant_gain_life(p), game.cant_lose_life(p)) {
            (true, true) => summary.pending_effects.push(format!("{} life total can't change.", if you { "Your" } else { "Their" })),
            (true, false) => summary.pending_effects.push(format!("{} can't gain life.", if you { "You" } else { "They" })),
            (false, true) => summary.pending_effects.push(format!("{} can't lose life.", if you { "You" } else { "They" })),
            (false, false) => {}
        }
        // "Damage can't be prevented this turn" (CR 615.12: Skullcrack, Stomp): a rule of the game for the turn, true of the
        // damage dealt to either player and their permanents, so shown beside each.
        if s.turn.no_prevention_turn == s.turn.number {
            summary.pending_effects.push("Damage can't be prevented this turn.".to_string());
        }
        // What keeps the player in the game, or out of winning it (CR 104.2b, 104.3), and how damage and counters reach them
        // (CR 702.90b, 122.1), from the static abilities that apply now: Platinum Angel, Phyrexian Unlife, Melira.
        let (they, them) = if p == self.viewer { ("You", "you") } else { ("They", "them") };
        let reasons = [LossReason::Life, LossReason::EmptyLibrary, LossReason::Poison, LossReason::Effect];
        if reasons.iter().all(|&r| game.doesnt_lose(p, r)) {
            summary.pending_effects.push(format!("{they} can't lose the game."));
        } else if game.doesnt_lose(p, LossReason::Life) {
            summary.pending_effects.push(format!("{they} don't lose the game for having 0 or less life."));
        }
        if game.cant_win(p) {
            summary.pending_effects.push(format!("{they} can't win the game."));
        }
        if game.damage_as_though_infect(p) {
            summary.pending_effects.push(format!("Damage is dealt to {them} as though its source had infect."));
        }
        if game.player_counters_banned(p, PlayerCounter::Poison) {
            summary.pending_effects.push(format!("{they} can't get poison counters."));
        }
        if player.room != 0
            && let Some(dungeon) = game.cards.undercity
        {
            let label = label_text(&Label::Card(dungeon, player.room - 1));
            summary.dungeon = "Undercity".into();
            summary.dungeon_room = label.split(" \u{2014} ").next().unwrap_or(label).to_string();
        }
        summary
    }

    /// "You can't cast spells from anywhere other than your hand until Bot1's next turn", as `viewer` reads a restriction on
    /// `p` (CR 101.2).
    fn cant_cast_text(&self, p: PlayerId, zones: ZoneKinds, duration: Duration) -> String {
        let (who, whose) = if p == self.viewer { ("You", "your") } else { ("They", "their") };
        let open: Vec<ZoneKind> = ZoneKinds::CAST_FROM.into_iter().filter(|&z| !zones.contains(z)).collect();
        let from = match open.as_slice() {
            [] => String::new(),
            [only] => format!(" from anywhere other than {whose} {}", only.name().to_lowercase()),
            _ => {
                let barred: Vec<String> =
                    ZoneKinds::CAST_FROM.into_iter().filter(|&z| zones.contains(z)).map(|z| z.name().to_lowercase()).collect();
                format!(" from {whose} {}", barred.join(" or "))
            }
        };
        let until = match duration {
            Duration::UntilNextTurnOf(q) if q == p && q != self.viewer => format!(" until {whose} next turn"),
            _ => self.until_text(duration),
        };
        format!("{who} can't cast spells{from}{until}.")
    }

    /// A prevention effect a spell or ability made (CR 615, 611.2), as the object or player it is about shows it: "Prevent the
    /// next 3 damage that would be dealt to it this turn (Healing Salve)." `whom` is "it", "you", "them" or empty for damage
    /// to anyone.
    fn prevention_text(&self, e: &ContinuousEffect, whom: &str) -> Option<String> {
        let EffectKind::Prevention { card, index, shield, .. } = e.kind else { return None };
        let card = self.game.cards.get(card);
        let damage = card.granted_preventions.get(index as usize)?.damage;
        let how = match shield {
            Shield::All => format!("Prevent all {damage}"),
            Shield::Next(n) => format!("Prevent the next {n} {damage}"),
        };
        let to = match card.granted_preventions.get(index as usize)?.of_it {
            Some(of_it) if whom == "it" => format!(" {of_it}"),
            _ if whom.is_empty() => String::new(),
            _ => format!(" to {whom}"),
        };
        Some(format!("{how} that would be dealt{to}{} ({}).", self.until_text(e.duration), card.name))
    }

    /// How long an effect lasts, as `viewer` reads it: " this turn", " until your next turn", " until Bot1's next turn", or
    /// nothing for one that lasts as long as its object.

    fn until_text(&self, duration: Duration) -> String {
        match duration {
            Duration::EndOfTurn => " this turn".to_string(),
            Duration::EndOfCombat => " this combat".to_string(),
            Duration::UntilNextTurnOf(q) if q == self.viewer => " until your next turn".to_string(),
            Duration::UntilNextTurnOf(q) => format!(" until {}'s next turn", self.names.get(q.index()).cloned().unwrap_or_default()),
            Duration::DuringControllersNextTurn(_) => " during its controller's next turn".to_string(),
            Duration::EndOfExtraTurn(_) => " during the extra turn".to_string(),
            Duration::WhileTapped => " for as long as it remains tapped".to_string(),
            Duration::WhileCounter(kind) => format!(" for as long as it has a {} counter on it", kind.name()),
            _ => String::new(),
        }
    }

    fn zone_view(&self, kind: ZoneKind, owner: PlayerId, handles: &mut HandleSpace, visible: &HashSet<ObjectId>) -> ZoneView {
        let s = &self.game.state;
        let mut contents: Vec<ObjectId> = match kind {
            ZoneKind::Library => s.zone(Zone::library(owner)).to_vec(),
            ZoneKind::Hand => s.zone(Zone::hand(owner)).to_vec(),
            ZoneKind::Graveyard => s.zone(Zone::graveyard(owner)).to_vec(),
            // The phased-out permanents too: players know of them though the game treats them as though they don't exist (CR
            // 702.26b), each shown as phased out.
            ZoneKind::Battlefield => s.battlefield().iter().chain(s.zone(Zone::PHASED_OUT)).copied().collect(),
            ZoneKind::Stack => stack_entries(s).iter().map(StackEntry::key).collect(),
            ZoneKind::Exile => s.seats().flat_map(|p| s.zone(Zone::exile(p)).to_vec()).collect(),
            ZoneKind::Command => s.zone(Zone::COMMAND).to_vec(),
            ZoneKind::Sideboard => s.zone(Zone::sideboard(owner)).to_vec(),
            ZoneKind::Gone | ZoneKind::PhasedOut => Vec::new(),
        };
        let count = contents.len() as i32;
        let zone = Zone::new(kind, owner);
        let contents_visible = looks_into(s, self.viewer, zone);
        let mut view = ZoneView {
            kind: Some(zone_kind_view(kind)),
            owner: zone_owner(zone),
            count,
            contents_visible,
            ..Default::default()
        };
        if contents_visible {
            contents.retain(|id| visible.contains(id));
            view.contents = contents.into_iter().map(|id| handles.handle(id)).collect();
        } else if kind == ZoneKind::Sideboard {
            // Another player's sideboard is closed, but the companion they revealed is in it for everyone to see (CR 103.2b):
            // its contents are that card alone, the count still the whole of it.
            view.contents = revealed_companion(s, owner).into_iter().map(|id| handles.handle(id)).collect();
        }
        view
    }

    fn combat(&self, handles: &mut HandleSpace) -> CombatView {
        let s = &self.game.state;
        let mut view = CombatView::default();
        for a in &s.attackers {
            let (target_kind, target_player, target_permanent) = match a.target {
                Target::Player(p) => (AttackTargetKindView::Player, seat(p), ViewHandle::NONE),
                Target::Object(id) => {
                    let kind = if self.game.chars(id).is(CardType::Battle) {
                        AttackTargetKindView::Battle
                    } else {
                        AttackTargetKindView::Planeswalker
                    };
                    (kind, 0, handles.handle(id))
                }
                _ => (AttackTargetKindView::Player, 0, ViewHandle::NONE),
            };
            view.attackers.push(AttackerView {
                attacker: handles.handle(a.id),
                target_kind,
                target_player,
                target_permanent,
                was_blocked: a.blocked,
                band: crate::prompt::wire(a.band),
            });
        }
        for a in &s.attackers {
            let blockers: Vec<ViewHandle> = s.blocks.iter().filter(|b| b.attacker == a.id).map(|b| handles.handle(b.blocker)).collect();
            if !blockers.is_empty() {
                view.blocks.push(BlockView { attacker: handles.handle(a.id), blockers });
            }
        }
        view
    }

    /// An ability on the stack, as a client draws it: the source's name and the ability's text.
    fn describe_ability(&self, item: &StackItem, handle: ViewHandle, handles: &mut HandleSpace) -> ObjectView {
        let game = self.game;
        let source = game.state.obj(item.object);
        let def = game.cards.get(game.chars(item.object).card);
        let text = game.cards.ability(item.ability).text();
        let mut view = ObjectView {
            handle,
            zone: Some(ZoneKindView::Stack),
            kind: ObjectKindView::AbilityOnStack,
            owner: seat(source.owner),
            controller: seat(item.controller),
            known: true,
            card_id: def.name.to_string(),
            name: def.name.to_string(),
            oracle_text: text.to_string(),
            ..Default::default()
        };
        for t in self.game.state.targets(item) {
            if let Some(message) = target_view(game, handles, *t) {
                view.targets.push(SpellTargetView {
                    player: message.player,
                    object: message.object,
                    text: String::new(),
                    still_legal: true,
                });
            }
        }
        view
    }

    /// What the layers gave an object's abilities and took from them (CR 613.1f), against the abilities it started with: the
    /// card's own, or a copy's. A keyword or ability it has that it did not start with is granted, and one it started with
    /// and has not is lost. The abilities a land has from its basic land types are the rules' and are neither. Nothing is
    /// allocated for an object the layers left alone, which is nearly every one.
    fn ability_changes(&self, view: &mut ObjectView, id: ObjectId, c: &Chars, copy: Option<Copiable>) {
        let cards = self.game.cards;
        let def = cards.get(c.card);
        let base = Keywords(def.printed_keywords().0 | copy.map_or(0, |copy| copy.extra_keywords.0));
        if c.keywords != base {
            for (keyword, words) in KEYWORD_WORDS {
                if c.keywords.has(keyword) && !base.has(keyword) {
                    view.granted_abilities.push(AbilityTextView { text: words.to_string(), spent: false });
                } else if base.has(keyword) && !c.keywords.has(keyword) {
                    view.lost_abilities.push(words.to_string());
                }
            }
        }
        // Toxic N (CR 702.164): its printed toxic is on its text, so what it has beyond that is granted, as one line of its
        // total (CR 702.164b), and printed toxic it has no more is lost - unless it lost all its abilities, which the walk of
        // its text below says, and after which all the toxic it has was granted.
        let printed_toxic = if c.lost_all { 0 } else { def.toxic };
        if c.toxic > printed_toxic {
            view.granted_abilities.push(AbilityTextView { text: format!("Toxic {}", c.toxic - printed_toxic), spent: false });
        } else if c.toxic < printed_toxic {
            view.lost_abilities.push(format!("Toxic {printed_toxic}"));
        }
        // "Hexproof from [quality]" (CR 702.11d): the qualities it has and did not start with are granted, those it started
        // with and has not are lost.
        if c.hexproof_from != def.hexproof_from {
            let granted = HexproofFrom(c.hexproof_from.0 & !def.hexproof_from.0);
            let lost = HexproofFrom(def.hexproof_from.0 & !c.hexproof_from.0);
            if !granted.is_empty() {
                view.granted_abilities.push(AbilityTextView { text: hexproof_from_text(granted), spent: false });
            }
            if !lost.is_empty() {
                view.lost_abilities.push(hexproof_from_text(lost));
            }
        }
        // Protection (CR 702.16) an effect or another permanent's static ability gave it (Mother of Runes, Akroma's Memorial):
        // no line of its own says so. One its own text prints (Progenitus, a leveled Hexdrinker, True-Name Nemesis) is that line.
        if !view.oracle_text.to_lowercase().contains("protection from") {
            if let Some(text) = self.protection_text(c) {
                view.granted_abilities.push(AbilityTextView { text, spent: false });
            }
        } else {
            // Its own text prints a protection: what resolved effects gave it besides (Mother of Runes on herself) is shown
            // on its own line, read from the effects. (A static ability of another permanent giving more is not told apart.)
            let mut given = *c;
            given.protection = ColorSet::NONE;
            given.protection_colorless = false;
            given.protection_players = 0;
            given.protection_types = TypeSet::NONE;
            given.protection_subtypes = SubtypeSet::NONE;
            given.protection_mana_value = None;
            given.protection_colored_spells = false;
            given.protection_everything = false;
            for e in self.game.state.effects.iter().filter(|e| e.affected == id) {
                match e.kind {
                    EffectKind::Protection(colors) => given.protection = ColorSet(given.protection.0 | colors.0),
                    EffectKind::ProtectionFromColorless => given.protection_colorless = true,
                    EffectKind::ProtectionFromTypes(types) => given.protection_types = TypeSet(given.protection_types.0 | types.0),
                    EffectKind::ProtectionFromEverything => given.protection_everything = true,
                    EffectKind::ProtectionFromPlayer(p) => given.protection_players |= 1 << p.0,
                    _ => {}
                }
            }
            if let Some(text) = self.protection_text(&given) {
                view.granted_abilities.push(AbilityTextView { text, spent: false });
            }
        }
        let printed = |ability: AbilityRef| {
            (ability.card == c.card && usize::from(ability.index) < def.abilities.len())
                // A Room's abilities are its doors' (CR 709.5): the layers give the unlocked ones.
                || cards.halves[c.card.0 as usize].is_some_and(|(left, right)| ability.card == left || ability.card == right)
                || copy.is_some_and(|copy| copy.adds_ability(ability))
        };
        for ability in c.abilities.iter().flatten().copied() {
            // An enchant ability (CR 702.5a) is on the card's text, or in the words of the effect that gave it.
            if ability.card != cards.intrinsic && !ability.is_enchant() && !printed(ability) {
                view.granted_abilities.push(AbilityTextView { text: cards.ability(ability).text().to_string(), spent: false });
            }
        }
        for index in 0..def.abilities.len().min(c.abilities.len()) {
            let ability = AbilityRef { card: c.card, index: index as u8 };
            if !c.abilities.contains(&Some(ability)) {
                view.lost_abilities.push(def.abilities[index].text().to_string());
            }
        }
        // Lost all its abilities (CR 613.1f): its static abilities, cost changes, replacement effects and other rules lines
        // are gone too, which no keyword or ability above names. A line of keywords alone, or one of the abilities above, is
        // judged above, so a keyword or ability had again after the loss is not called lost.
        if Query::lost_all_abilities(self.game, id) {
            // A line that is reminder text alone ("({T}: Add {R} or {G}.)") reminds of the abilities of a basic land type
            // (CR 305.6): lost only if the land no longer has one, as under Humility, not under Blood Sun's "except mana
            // abilities", which keeps them.
            let intrinsic = c.abilities().any(|a| a.card == cards.intrinsic);
            let judged = |line: &str| {
                let words = without_reminder(line);
                if words.trim().is_empty() && intrinsic {
                    return true;
                }
                let keyword = |part: &str| {
                    KEYWORD_WORDS.iter().any(|(_, w)| w.trim_end_matches('.').eq_ignore_ascii_case(part.trim().trim_end_matches('.')))
                };
                // "Hexproof from [quality]" is judged with the qualities above.
                (!words.is_empty() && words.split(',').all(keyword))
                    || words.starts_with("Hexproof from")
                    || def.abilities.iter().any(|a| without_reminder(a.text()) == words)
            };
            let rest: Vec<String> =
                view.oracle_text.split('\n').filter(|line| !line.trim().is_empty() && !judged(line)).map(str::to_string).collect();
            view.lost_abilities.extend(rest);
        }
    }

    /// The qualities an object has protection from (CR 702.16a), worded as printed cards word them: "Protection from
    /// everything", "Protection from black and from red", "Protection from creatures", "Protection from <player>". None when
    /// it has none.
    fn protection_text(&self, c: &Chars) -> Option<String> {
        protection_words(c, self.names)
    }

    /// A standing permission to play a card, as the viewer reads it: who may, what, and until when.
    fn permission_text(&self, permission: &PlayPermission) -> String {
        let mine = permission.player == self.viewer;
        let (prefix, whose) = if mine {
            (String::new(), "your".to_string())
        } else {
            let name = self.names.get(permission.player.index()).cloned().unwrap_or_default();
            (format!("{name}: "), "their".to_string())
        };
        let grant = permission.grant;
        let window = match grant.expiry {
            Expiry::Never => "while it stays here".to_string(),
            Expiry::EndOfTurn => "this turn".to_string(),
            Expiry::EndOfYourNextTurn => format!("until {whose} next turn ends"),
            Expiry::UntilYourNextEndStep => format!("until {whose} next end step"),
            // Named as the permanent is now: a transformed one by its other face (CR 712.18).
            Expiry::WhileControls(source) => {
                let name = self.game.cards.get(self.game.chars(source).card).name;
                format!("for as long as {} control {name}", if mine { "you" } else { "they" })
            }
        };
        let mut text = format!("{prefix}{} {window}", if grant.lands { "Playable" } else { "Castable" });
        // A condition the permission stands under is said, so one not met reads as the conditional it is.
        let who = if mine { "you" } else { "they" };
        match grant.when {
            Some(PlayCondition::AttackedWithThisTurn(subtype)) => text.push_str(&format!(", if {who} attacked with a {}", subtype.name())),
            Some(PlayCondition::ControlsSubtype(subtype)) => text.push_str(&format!(", if {who} control a {}", subtype.name())),
            None => {}
        }
        if grant.adventure_only {
            text.push_str(", as an Adventure");
        }
        if grant.sorcery_speed {
            text.push_str(", as a sorcery");
        }
        if grant.later_turns {
            text.push_str(", on a later turn");
        }
        if grant.flashback {
            text.push_str(", then exiled");
        }
        if grant.exile_instead != ExileInstead::Never {
            text.push_str(", exiled instead of put into the graveyard");
        }
        // What the permission adds to playing it, for what the card can be played as: a spell's extra cost (CR 601.2f), a land's
        // entering tapped (CR 614.1c). A modal double-faced card may be either (CR 712.3, 712.12).
        let chars = self.game.chars(permission.card);
        let def = self.game.cards.get(chars.card);
        let land = chars.is(CardType::Land) || def.back_land;
        let spell = !chars.is(CardType::Land) || def.back.is_some_and(|b| b.has_cost);
        if grant.more > 0 && spell {
            text.push_str(&format!(", a spell costs {{{}}} more", grant.more));
        }
        if grant.life_for_mana_value && spell {
            text.push_str(", a spell for life equal to its mana value");
        }
        if grant.lands && grant.enters_tapped && land {
            text.push_str(", a land enters tapped");
        }
        text
    }

    fn describe(&self, id: ObjectId, handle: ViewHandle, item: Option<StackItem>, handles: &mut HandleSpace) -> ObjectView {
        let game = self.game;
        let s = &game.state;
        let obj = s.obj(id);
        let c = game.chars(id);
        let def = game.cards.get(c.card);
        // A phased-out permanent is shown as the permanent it still is (CR 702.26d), marked phased out.
        let phased_out = obj.zone.is(ZoneKind::PhasedOut);
        let on_battlefield = obj.zone.is(ZoneKind::Battlefield) || phased_out;
        let zone = if item.is_some() { Zone::STACK } else { obj.zone };

        let mut counters: Vec<CounterView> = CounterKind::ALL
            .iter()
            .filter(|k| obj.counters(**k) > 0)
            .map(|k| CounterView { name: k.name().to_string(), count: i32::from(obj.counters(*k)) })
            .collect();
        counters.sort_by(|a, b| a.name.cmp(&b.name));

        // What was chosen for it that the text it has now refers to (CR 607.2d, 607.5): a choice made by a text it no longer
        // has as its own - before it became a copy of something else - is no part of the permanent shown.
        let mut choices = Vec::new();
        let text = game.cards.linked_to(s, id);
        if let Some(color) = s.chosen_color(text) {
            choices.push(ChoiceView { key: "color".into(), value: color.name().to_string(), ..Default::default() });
        }
        if let Some(sub) = s.chosen_subtype(text) {
            choices.push(ChoiceView { key: "creature type".into(), value: sub.name().to_string(), ..Default::default() });
        }
        if let Some(p) = s.chosen_player(text) {
            choices.push(ChoiceView { key: "player".into(), value: seat(p).to_string(), ..Default::default() });
        }
        if let Some(card) = s.chosen_card(text) {
            choices.push(ChoiceView { key: "name".into(), value: game.cards.get(card).name.to_string(), ..Default::default() });
        }
        // One of the words its text offered to choose between as it entered (CR 607.2f), by the word: "Jeskai", "Odd".
        if let Some(chosen) = s.chosen_option(text) {
            choices.push(ChoiceView { key: "option".into(), value: label_text(&Label::Option(chosen.card, chosen.index)).to_string(), ..Default::default() });
        }
        // The permanent chosen as it entered (CR 614.12a, 607.2d), while it is still that permanent on the battlefield: one that
        // left, or came back as a new object (CR 400.7), is chosen no longer. Named as it shows there, and pointed at.
        if let Some(chosen) = s.chosen_object(text)
            && s.obj(chosen).became.is_none()
            && s.obj(chosen).zone.is(ZoneKind::Battlefield)
        {
            let shown = game.chars(chosen);
            let key = if shown.is(CardType::Creature) { "creature" } else { "permanent" };
            let value = game.cards.get(shown.card).name.to_string();
            choices.push(ChoiceView { key: key.into(), value, object: handles.handle(chosen) });
        }
        choices.sort_by(|a, b| a.key.cmp(&b.key));

        let effect = |kind: EffectKind| s.effects.iter().any(|e| e.affected == id && e.kind == kind);
        let mut view = ObjectView {
            handle,
            zone: Some(zone_kind_view(zone.kind())),
            zone_owner: zone_owner(zone),
            kind: if obj.card_copy {
                ObjectKindView::CardCopy
            } else if obj.token {
                ObjectKindView::Token
            } else if zone.is(ZoneKind::Command) {
                ObjectKindView::Emblem
            } else {
                ObjectKindView::Card
            },
            owner: seat(obj.owner),
            controller: seat(obj.controller),
            known: true,
            card_id: def.name.to_string(),
            name: def.name.to_string(),
            mana_cost: cost_text(def),
            mana_value: Query::mana_value(game, id) as i32,
            oracle_text: crate::text::changed_text(def.text, def.name, if phased_out { current_words(s, id) } else { Query::words(game, id) }),
            supertypes: supertypes_view(c.supertypes),
            card_types: types_view(c.types),
            subtypes: subtypes_view(c.subtypes),
            colors: colors_view(c.colors),
            power: c.power.map(i32::from),
            toughness: c.toughness.map(i32::from),
            tapped: obj.tapped,
            damage_marked: i32::from(obj.damage),
            counters,
            summoning_sick: on_battlefield && summoning_sick(game.cards, s, &game.log, id),
            produces_mana: c.abilities().any(|a| matches!(game.cards.ability(a), AbilityDef::Activated(d) if d.is_mana_ability())),
            monstrous: obj.marks.monstrous,
            solved: on_battlefield && obj.marks.solved,
            suspected: on_battlefield && effect(EffectKind::Suspected),
            phased_out,
            station_pt: cantrip_oracle::station_pt(def.name).unwrap_or_default().to_string(),
            exerted: obj.marks.exerted_by.is_some(),
            saddled: game.is_saddled(id),
            goaded: effect(EffectKind::Goaded),
            band: s.attackers.iter().find(|a| a.id == id).map_or(0, |a| crate::prompt::wire(a.band)),
            harnessed: effect(EffectKind::Harnessed),
            choices,
            class_level: if on_battlefield { i32::from(obj.class_level) } else { 0 },
            // A token whose text was changed as it was made has copiable values of its own, and the name the changed text
            // gives it, and is no copy (CR 612.4, 111.4).
            // A prototyped spell or permanent has its own card's prototyped face as its copiable values (CR 718.2a, 718.3b),
            // and is no copy of anything.
            is_copy: obj.copiable().is_some_and(|c| {
                let name = if !c.words.is_printed() && c.name == game.cards.get(c.card).token_name_under(c.words) { None } else { c.name };
                let own_prototype = prototyped_face(game.cards.get(obj.card)).is_some_and(|f| std::ptr::eq(f, game.cards.get(c.card)));
                let values = Copiable { words: Words::NONE, name, ..c };
                values != Copiable::of(obj.card) && !(own_prototype && values == Copiable::of(c.card))
            }) || obj.card_copy,
            // A copy's exception may state its loyalty (CR 707.9b: "has starting loyalty X").
            starting_counter: if on_battlefield { None } else { obj.copiable().and_then(|c| c.stated_loyalty()).or(def.loyalty).map(i32::from) },
            other_face: def.back.map(|back| Box::new(printed(back))),
            // A split card as it is printed, its halves side by side (CR 709.1). On the stack a half cast alone is that half
            // only, whose definition has no halves (CR 709.3b), and a fused split spell is both (CR 702.102b).
            halves: def.split.map_or_else(Vec::new, |(left, right)| vec![printed(left), printed(right)]),
            // Cast as its Adventure or Omen, it is that face alone (CR 715.3b, 720.3b), whose definition has no inset.
            inset_spell: inset_spell(def),
            // The prototype line is part of the card, and of its prototyped face (CR 718.2a).
            prototype: def.prototype.map_or_else(String::new, |p| p.text.to_string()),
            prototyped: def.prototyped,
            ..Default::default()
        };

        // A name an effect set (CR 612.8) or a copy effect states (CR 707.9b) is the name it has.
        if c.name_set || obj.copiable().is_some_and(|copy| copy.name.is_some()) {
            view.name = c.name.to_string();
        }
        if def.is_room() && on_battlefield && !c.name_set {
            // A Room on the battlefield (CR 709.5) is its unlocked doors: their names, costs and text; a locked door is
            // one the client can show as locked.
            let doors = obj.marks.doors;
            view.name = c.name.to_string();
            // With neither door unlocked there is no mana cost at all (CR 709.5), not {0}.
            view.mana_cost = if doors & (DOOR_LEFT | DOOR_RIGHT) == 0 { String::new() } else { def.room_cost(doors).to_string() };
            // The doors' text, read through the words it has now (CR 612).
            let text = def.unlocked_doors(doors).map(|half| half.text).collect::<Vec<_>>().join("
");
            view.oracle_text = crate::text::changed_text(&text, def.name, Query::words(game, id));
            view.unlocked_doors = vec![doors & DOOR_LEFT != 0, doors & DOOR_RIGHT != 0];
        }

        // Every creature type (Changeling): the type line prints the types the card has of its own, and says the rest.
        if (c.is(CardType::Creature) || c.is(CardType::Kindred))
            && Subtype::CREATURE_TYPES.iter().all(|&t| c.subtypes.has(t))
        {
            view.every_creature_type = true;
            view.type_line_subtypes =
                c.subtypes.iter().filter(|t| !t.is_creature_type() || def.subtypes.has(*t)).map(|t| t.name().to_string()).collect();
        }
        view.cant_be_countered = obj.marks.cant_be_countered;
        // A Case's two keyword lines (CR 719.3): once it is solved its "To solve" ability can do nothing more (CR 719.3a), and
        // its "Solved" ability may affect the game only while it is (CR 719.3c), so the line that can't is greyed.
        if on_battlefield && c.subtypes.has(Subtype::Case) {
            for (i, line) in view.oracle_text.split('\n').enumerate() {
                let idle = if line.starts_with("To solve") {
                    obj.marks.solved
                } else {
                    line.starts_with("Solved") && !obj.marks.solved
                };
                if idle {
                    view.spent_lines.push(i as i32);
                }
            }
        }
        for permission in s.permissions.iter().filter(|p| p.card == id) {
            view.play_permissions.push(PlayPermissionView {
                seat: seat(permission.player),
                text: self.permission_text(permission),
            });
        }
        // A permanent's permission to cast a card in exile (CR 601.3), while it would allow the cast: what it adds to the cost
        // ("Cast by removing three counters from among creatures you control").
        if zone.is(ZoneKind::Exile) && !s.battlefield().is_empty() {
            for (player, text) in exile_cast_texts(game, id) {
                let prefix = if player == self.viewer { String::new() } else { format!("{}: ", self.names.get(player.index()).cloned().unwrap_or_default()) };
                view.play_permissions.push(PlayPermissionView { seat: seat(player), text: format!("{prefix}{text}") });
            }
        }
        if (on_battlefield || zone.is(ZoneKind::Stack)) && !obj.face_down {
            self.ability_changes(&mut view, id, &c, obj.copy);
        }
        // "Can't attack", "can't block", "can't be blocked [except by ...]" and "blocks if able" from a resolved effect (CR 508.1c, 509.1b-c), with how long they last: no ability
        // of the creature says so, so its face does.
        if on_battlefield {
            for e in s.effects.iter().filter(|e| e.affected == id) {
                let what = match e.kind {
                    EffectKind::CantAttack => "Can't attack",
                    EffectKind::CantBlock => "Can't block",
                    EffectKind::CantBeBlocked => "Can't be blocked",
                    // A resolved restriction on blocking it, in its card's words: "Can't be blocked except by creatures with
                    // haste this turn."
                    EffectKind::BlockRestriction { card, index } => match game.cards.get(card).lasting_block_rules.get(index as usize) {
                        Some(rule) => rule.text,
                        None => continue,
                    },
                    // A requirement to block (CR 509.1c): "Blocks this turn if able."
                    EffectKind::MustBlock => {
                        let until = match self.until_text(e.duration) {
                            when if when.is_empty() => " each combat".to_string(),
                            when => when,
                        };
                        view.granted_abilities.push(AbilityTextView { text: format!("Blocks{until} if able."), spent: false });
                        continue;
                    }
                    // A requirement to block one creature (CR 509.1c, 702.39a): "Blocks Goblin Grappler this combat if able."
                    EffectKind::MustBlockAttacker(attacker) => {
                        let until = self.until_text(e.duration);
                        view.granted_abilities.push(AbilityTextView { text: format!("Blocks {}{until} if able.", game.name(attacker)), spent: false });
                        continue;
                    }
                    // A requirement to attack (CR 508.1d), and what: "Attacks Gideon, Battle-Forged during its controller's next
                    // turn if able."
                    EffectKind::MustAttackAt(at) => {
                        let whom = match at {
                            Target::Player(p) if p == self.viewer => "you".to_string(),
                            Target::Player(p) => self.names.get(p.index()).cloned().unwrap_or_default(),
                            Target::Object(o) => game.name(o).to_string(),
                            _ => continue,
                        };
                        view.granted_abilities.push(AbilityTextView { text: format!("Attacks {whom}{} if able.", self.until_text(e.duration)), spent: false });
                        continue;
                    }
                    EffectKind::CantActivateAbilities => "Its activated abilities can't be activated",
                    EffectKind::CantBeSacrificed => "Can't be sacrificed",
                    EffectKind::CantBecomeUntapped => "Can't become untapped",
                    // A resolved control-changing effect (CR 613.1b): who has it and for how long, named for where it came
                    // from, as nothing else on the permanent says so. A permanent given for good says nothing.
                    EffectKind::Control(p) if e.duration != Duration::Indefinite => {
                        let who = if p == self.viewer { "You control".to_string() } else { format!("{} controls", self.names.get(p.index()).cloned().unwrap_or_default()) };
                        let from = game.cards.get(game.chars(e.source).card).name;
                        let until = match e.duration {
                            Duration::AsLongAs(card, index) => game.cards.get(card).lasting.get(index as usize).map_or(String::new(), |l| format!(" for as long as {}", l.text.replace("{source}", from))),
                            other => self.until_text(other),
                        };
                        view.granted_abilities.push(AbilityTextView { text: format!("{who} it{until} ({from})."), spent: false });
                        continue;
                    }
                    // A prevention shield on it (CR 615.7), or all damage to it prevented: no ability of its own says so.
                    // Or one about it that names more than the damage dealt to it (Maze of Ith's "to and dealt by").
                    EffectKind::Prevention { to, card, index, .. }
                        if to == Target::Object(id)
                            || (to == Target::None
                                && game.cards.get(card).granted_preventions.get(index as usize).is_some_and(|d| d.of_it.is_some())) =>
                    {
                        if let Some(text) = self.prevention_text(e, "it") {
                            view.granted_abilities.push(AbilityTextView { text, spent: false });
                        }
                        continue;
                    }
                    _ => continue,
                };
                view.granted_abilities.push(AbilityTextView { text: format!("{what}{}.", self.until_text(e.duration)), spent: false });
            }
            // "Can't attack" from a static ability while its condition holds (CR 508.1c: Patchwork Beastie without delirium).
            if c.is(CardType::Creature) && !game.attack_rules_allow(id) {
                view.granted_abilities.push(AbilityTextView { text: "Can't attack.".to_string(), spent: false });
            }
            // A creature with defender that may attack as though it didn't have it right now (CR 702.3b, 609.4): by a resolved
            // effect, with how long it lasts (Assault Formation), or while a static ability's condition holds (Demon Wall with a
            // counter on it). It still has defender, so its face says both.
            if c.has(Keyword::Defender) && c.is(CardType::Creature) {
                const TEXT: &str = "Can attack as though it didn't have defender";
                if let Some(e) = s.effects.iter().find(|e| e.affected == id && e.kind == EffectKind::CanAttackDespiteDefender) {
                    view.granted_abilities.push(AbilityTextView { text: format!("{TEXT}{}.", self.until_text(e.duration)), spent: false });
                } else if game.may_attack_despite_defender(id) {
                    view.granted_abilities.push(AbilityTextView { text: format!("{TEXT}."), spent: false });
                }
            }
        }

        if obj.face_down {
            // Face down (CR 708.2a, 406.3): what it is face down goes to every seat, a nameless 2/2 on the battlefield and on
            // the stack (a spell cast face down by morph or disguise, CR 708.4) and no characteristics at all in exile; what it
            // really is only to a seat that may look at it, its controller for a permanent or a spell (CR 708.5) and those who
            // saw it go there for a card (CR 406.3), and to a seat it is being shown to (a reveal, CR 701.20a, 701.40g).
            let looks = knows_face(s, self.viewer, id) || self.faces.contains(&id);
            if !on_battlefield && !zone.is(ZoneKind::Stack) {
                view.name = String::new();
                view.mana_cost = String::new();
                view.mana_value = 0;
                view.oracle_text = String::new();
                view.supertypes.clear();
                view.card_types.clear();
                view.subtypes.clear();
                view.colors.clear();
                view.power = None;
                view.toughness = None;
                view.produces_mana = false;
                view.other_face = None;
                view.inset_spell = None;
                view.starting_counter = None;
                view.station_pt = String::new();
                // Nothing the layers or the printed card say about it crosses either (CR 406.3): not that it is every creature
                // type, nor what it was granted or has lost.
                view.every_creature_type = false;
                view.type_line_subtypes.clear();
                view.granted_abilities.clear();
                view.lost_abilities.clear();
                view.cant_be_countered = false;
                view.halves.clear();
            }
            let real = game.cards.get(obj.card).name;
            view.face_down = true;
            view.is_copy = obj.card_copy;
            view.known = looks;
            view.card_id = if looks { real.to_string() } else { String::new() };
            view.face_down_name = if looks { real.to_string() } else { String::new() };
        }
        if on_battlefield && let Some(host) = game.attached_to(id) {
            view.attached_to = handles.handle(host);
        }
        // An Aura or Equipment that phased out with what it is attached to is still attached to it (CR 702.26g).
        if phased_out
            && let Some(host) = obj.attached_to
            && live(s, host)
            && s.obj(host).zone.is(ZoneKind::PhasedOut)
        {
            view.attached_to = handles.handle(host);
        }
        if obj.zone.is(ZoneKind::Exile)
            && let Some(link) = obj.exiled
            && live(s, link.by)
            && s.obj(link.by).zone.is(ZoneKind::Battlefield)
        {
            view.exiled_with = handles.handle(link.by);
        }
        if let Some(item) = item {
            // A fused split spell's targets each say whose half they are (CR 702.102b): its left half's specs come first.
            let half_of = |slot: usize| -> &'static str {
                match def.split {
                    Some((left, right)) if item.way == WAY_FUSED => {
                        if (s.spec(&item, slot) as usize) < left.spell_targets().len() { left.name } else { right.name }
                    }
                    _ => "",
                }
            };
            for (slot, t) in s.targets(&item).iter().enumerate() {
                if let Some(message) = target_view(game, handles, *t) {
                    view.targets.push(SpellTargetView {
                        player: message.player,
                        object: message.object,
                        text: half_of(slot).to_string(),
                        still_legal: true,
                    });
                }
            }
            if let Some(modal) = def.modal {
                for (i, mode) in modal.modes.iter().enumerate() {
                    if modal.times(item.modes, i) > 0 {
                        view.chosen_modes.push(mode.text.to_string());
                    }
                }
            }
        }
        view
    }
}

/// A count of mana in a pool, which has no limit (CR 106.4), as the protocol's `int`: past `i32::MAX` it shows that.
fn count(n: u32) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// The words of an object's protection (CR 702.16a), `names` being the seats' names.
fn protection_words(c: &Chars, names: &[String]) -> Option<String> {
    if c.protection_everything {
        return Some("Protection from everything".to_string());
    }
    let mut words: Vec<String> = c.protection.iter().map(|color| color.name().to_lowercase()).collect();
    if c.protection_colorless {
        words.push("colorless".to_string());
    }
    if c.protection_colored_spells {
        words.push("colored spells".to_string());
    }
    // Plural, as cards print it: "from instants", "from sorceries".
    words.extend(c.protection_types.iter().map(|t| {
        let name = t.name().to_lowercase();
        match name.strip_suffix('y') {
            Some(stem) => format!("{stem}ies"),
            None => format!("{name}s"),
        }
    }));
    // A creature type keeps its capital, as printed: "from Dragons", "from Elves".
    words.extend(c.protection_subtypes.iter().map(|t| crate::text::plural(t.name())));
    if let Some(least) = c.protection_mana_value {
        words.push(format!("mana value {least} or greater"));
    }
    for p in 0..8u8 {
        if c.protection_players & (1 << p) != 0 {
            words.push(names.get(p as usize).cloned().unwrap_or_else(|| format!("player {}", p + 1)));
        }
    }
    match words.len() {
        0 => None,
        1 => Some(format!("Protection from {}", words[0])),
        2 => Some(format!("Protection from {} and from {}", words[0], words[1])),
        n => Some(format!("Protection from {}, and from {}", words[..n - 1].join(", from "), words[n - 1])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A power and toughness printed as stars are not a number on the printed card: the goyfs' sizes are their abilities'
    /// (CR 208.2a), and what the stars are worth once that is lost is the ability's (`StaticDef::lost_pt`), so the card as read
    /// in a deck builder or a booster shows none.
    #[test]
    fn a_printed_card_with_stars_for_power_and_toughness_shows_neither() {
        let cards = cantrip_engine::cards();
        for name in ["Tarmogoyf", "Nethergoyf", "Barrowgoyf", "Pyrogoyf"] {
            let view = printed(cards.get(cards.find(name).expect("the card")));
            assert_eq!((view.power, view.toughness), (None, None), "{name}");
        }
    }

    /// "Hexproof from [quality]" granted to a card is worded as printed cards word it (CR 702.11d, 702.11f-g).
    #[test]
    fn hexproof_from_is_worded_as_cards_print_it() {
        let black = HexproofFrom::color(Color::Black);
        let white = HexproofFrom::color(Color::White);
        assert_eq!(hexproof_from_text(HexproofFrom::MONOCOLORED), "Hexproof from monocolored");
        assert_eq!(hexproof_from_text(white.and(black)), "Hexproof from white and from black");
        assert_eq!(hexproof_from_text(HexproofFrom::EACH_COLOR), "Hexproof from each color");
        let types = HexproofFrom::card_type(CardType::Artifact)
            .and(HexproofFrom::card_type(CardType::Creature))
            .and(HexproofFrom::card_type(CardType::Enchantment));
        assert_eq!(hexproof_from_text(types), "Hexproof from artifacts, creatures, and enchantments");
    }

    /// Protection from a creature type or a mana value, granted, is shown as it is printed (CR 702.16a).
    #[test]
    fn protection_from_creature_types_and_a_mana_value_is_worded() {
        let cards = cantrip_engine::cards();
        let deck = vec![cards.find("Plains").expect("the card"); 20];
        let game = cantrip_engine::Game::new(cards, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
        let id = game.state.zone(Zone::library(PlayerId(0)))[0];
        let mut c = game.chars(id);
        c.protection_subtypes = SubtypeSet::of(&[Subtype::Dragon]);
        assert_eq!(protection_words(&c, &[]).as_deref(), Some("Protection from Dragons"));
        c.protection_subtypes = SubtypeSet::of(&[Subtype::Elf, Subtype::Dragon]);
        c.protection_mana_value = Some(3);
        let text = protection_words(&c, &[]).expect("protection");
        assert!(text.contains("Dragons") && text.contains("Elves") && text.ends_with("and from mana value 3 or greater"), "{text}");
        c.protection_subtypes = SubtypeSet::NONE;
        assert_eq!(protection_words(&c, &[]).as_deref(), Some("Protection from mana value 3 or greater"));
    }
}
