//! The rows the state is made of. Every one is `Copy`: plain data that names everything else by id.

use crate::vocab::*;

/// One object (CR 109): a card or token in some zone, or a copy of a card on the stack.
///
/// Characteristics are not stored here beyond the card it is: they are computed from the card, the copy effect it
/// entered with and the continuous effects that apply to it (CR 613), when asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Obj {
    pub card: CardId,
    pub owner: PlayerId,
    /// Who controls it. A move to another zone makes a new object (CR 400.7) and leaves this row as it was, so the row of
    /// an object that has left the battlefield keeps the player who last controlled it there: its last-known controller
    /// (CR 113.7a, 608.2h), by which a departed damage source's protection, lifelink and `Event::Damage` are judged.
    pub controller: PlayerId,
    pub zone: Zone,
    pub token: bool,
    pub tapped: bool,
    /// The turn it came under its controller's control: summoning sick (CR 302.6) while that is not before the
    /// turn its controller's most recent turn began on.
    pub controlled_since: u32,
    pub damage: u16,
    /// Dealt damage by a source with deathtouch since the last state-based action check (CR 704.5h).
    pub deathtouched: bool,
    /// The counters on it (CR 122). Private: they are read through `counters` and `counter_array`, taken off through
    /// `remove_counters`, and put on only by the put path, `cantrip_rules::counters`, which names the player who puts them
    /// (CR 122.6a) and lets the replacement effects of counters being put apply (CR 614.1a).
    counters: [i16; CounterKind::COUNT],
    /// When it entered its zone (CR 613.7d), for ordering and for layer timestamps.
    pub timestamp: u32,
    /// The copy effects on it (CR 707.2, 613.2, layer 1a): the values it copied, which are its copiable values unless it
    /// is face down (`copiable`, layer 1b). An object about to be put onto the battlefield face down holds the face-down
    /// values here until it enters (`put_onto_battlefield_face_down`).
    pub copy: Option<Copiable>,
    /// The turn a loyalty ability of it was last activated (CR 606.3), zero for never.
    pub loyalty_turn: u32,
    /// The new object it became when it changed zones (CR 400.7), so that a reference to the old one can be
    /// followed where the rules allow ("exile that card"). No rule follows it past one move: an effect finds the object a
    /// card or spell became where it went (CR 400.7a-c, 608.2h, dash's and warp's "it"), never what that object became in
    /// turn. The loop watch's fingerprint relies on it (`cantrip_rules::loops`, `Canon::id`): past one move a reference is
    /// named only by what it still says.
    pub became: Option<ObjectId>,
    /// Its last-known information, once it has ceased to be this object (CR 400.7, 608.2h, 113.7a): the row of
    /// `GameState::last_known` taken as it changed zones or ceased to exist, whatever zone it left.
    pub last_known: Option<u32>,
    /// If it would leave the battlefield, it is exiled instead (unearth, CR 702.84a; and the like).
    pub exile_on_leave: bool,
    /// How many times its kicker (or squad, or multikicker) cost was paid as it was cast (CR 702.33d), carried from
    /// the spell to the permanent it becomes.
    pub kicked: u8,
    /// How many times its `optional_cost` (an additional cost that is not kicker: bargain, teamwork, behold, blight,
    /// CR 601.2b) was paid as it was cast, carried from the spell to the permanent it becomes. Kept apart from `kicked`
    /// so a card with kicker and another optional cost is not ambiguous.
    pub paid: u32,
    /// The opponent its gift was promised to as it was cast (CR 702.174k), carried from the spell to the permanent it
    /// becomes: "if its gift cost was paid" (CR 702.174b). A copy of the spell has it too (CR 707.10); a permanent
    /// that enters as a copy of one does not (CR 707.9).
    pub gift: Option<PlayerId>,
    /// The mana spent to cast it, by kind (CR 601.2h): what converge and "if at least two of a color were spent"
    /// read. Carried from the spell to the permanent; a copy has none.
    pub spent: ManaTally,
    /// Which way it was cast (0 its mana cost, n its nth alternative cost), carried to the permanent: evoke's "if
    /// its evoke cost was paid".
    pub cast_way: u8,
    /// A Class's level (CR 716): 0 for never set, which reads as 1.
    pub class_level: u8,
    /// Prepared (CR 722.1): while it is, its controller may cast a copy of its prepare spell.
    pub prepared: bool,
    /// A copy of a card (CR 707.12, 722.3): not a card; it ceases to exist anywhere but where it may be, and leaves
    /// the stack for nowhere.
    pub card_copy: bool,
    /// A token made as a copy of one face of a card that has two - a copy of a Clone that copied a double-faced permanent
    /// (CR 707.8a) - is single-faced, whatever copy effect later applies to it.
    pub one_face: bool,
    /// For a prepare spell's copy in exile: the prepared permanent it belongs to, for as long as its exemption lasts
    /// (CR 722.3c). The link is severed for good when that permanent becomes unprepared or phases out (CR 702.26f), even
    /// if it prepares again or phases in before the next state-based check ends the copy (CR 704.5e).
    pub prepared_from: Option<ObjectId>,
    /// Exiled "on an adventure" (CR 715.3d): by the player who controlled the Adventure spell as it resolved, who "may play
    /// it" for as long as it remains exiled - its owner's exile zone holds it, whoever that player is.
    pub on_adventure: Option<PlayerId>,
    /// For a spell: the kind of zone it was cast from (CR 601.2a), what rebound ("cast from your hand") and "whenever
    /// you cast a spell from exile" ask. Not carried on when it leaves the stack.
    pub cast_from: Option<ZoneKind>,
    /// The turn some of its activated abilities were last activated, and which of them (bit n is the nth of its
    /// computed ability list): "activate only once each turn" (CR 602.5b-c), counted for this object and for each
    /// instance of the ability on its own.
    pub activated_turn: u32,
    pub activated_mask: u32,
    /// What it is attached to (CR 301.5, 303.4): an Aura's or Equipment's host. Only meaningful while the host is
    /// still the object it was (`live`) and where an attachment may be; state-based actions clear it otherwise
    /// (CR 704.5m-n). Kept on the row of an object that left, as last-known information.
    pub attached_to: Option<ObjectId>,
    /// The player an Aura with "enchant player" enchants (CR 303.4, 702.5): read by "enchanted player".
    pub attached_player: Option<PlayerId>,
    /// The Aura it was put onto the battlefield with and attached to (Animate Dead's "enchant creature put onto the
    /// battlefield with this Aura").
    pub entered_with: Option<ObjectId>,
    /// The X its spell was cast with (CR 107.3m), carried to the permanent: an enters trigger that reads X.
    pub x: u16,
    /// Designations and choices a permanent carries that no counter or characteristic holds.
    pub marks: Marks,
    /// A double-faced permanent with its back face up (CR 711.2, 712.3): its characteristics are the back face's.
    pub transformed: bool,
    /// For a card in exile: the permanent whose ability exiled it, which a linked ability finds it by (CR 607.2a).
    /// A new object (CR 400.7) has none.
    pub exiled: Option<ExileLink>,
    /// Face down (CR 708): a permanent is a nameless 2/2 creature, its card's real values kept in `card` and any copy
    /// effects in `copy` (the face-down values apply over them, `Marks::face_down_as`, CR 708.2a, 613.2b); a card in exile
    /// has no characteristics at all (CR 406.3a), and only those who may look
    /// at it know what card it is (`seen_by`, CR 406.3).
    pub face_down: bool,
    /// For a face-down card in exile: the seats that may look at it, a bit each (CR 406.3), every seat a game deals
    /// (`MAX_SEATS`). Those who could see where it came from, and those it was shown to since.
    pub seen_by: u8,
}

/// What a permanent is marked with: exerted (CR 701.43), monstrous (CR 701.37), solved (CR 719.3b), saddled (CR 702.171), a player chosen
/// as it entered (CR 614.12a), and whose it was before an effect changed its controller (CR 613.1b). All of it ends
/// when the object does: a permanent that changes zones is a new object (CR 400.7) with none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Marks {
    /// The unlocked designations of a permanent with a shared type line (CR 709.5c): `DOOR_LEFT` is "left half unlocked",
    /// `DOOR_RIGHT` "right half unlocked". Given as it enters (CR 709.5d) or by an unlock (CR 709.5e-f); a new object
    /// (CR 400.7) has neither.
    pub doors: u8,
    /// The player whose next untap step it sits out (CR 701.43a).
    pub exerted_by: Option<PlayerId>,
    /// It has become monstrous (CR 701.37b).
    pub monstrous: bool,
    /// It has become solved (CR 719.3b): a designation, neither an ability nor a copiable value, kept until it leaves the
    /// battlefield. A permanent that becomes a copy of a solved Case is not solved; a solved Case that becomes a copy of
    /// another Case stays solved.
    pub solved: bool,
    /// The player protecting a battle (CR 310.9): chosen as it entered (CR 310.9a), the defending player of what
    /// attacks it (CR 310.9d) and the only one who may block that.
    pub protector: Option<PlayerId>,
    /// A creature spell paid for with mana that makes it gain haste (CR 106.6): the permanent it becomes has haste
    /// until end of turn.
    pub gains_haste: bool,
    /// An ability the permanent a spell becomes gains (a granted way's "if you do, it gains ...", Serra Paragon): a delayed
    /// ability of the card that gave the way, added as the spell resolves.
    pub gains: Option<AbilityRef>,
    /// A spell cast at a time a sorcery could not have been cast (CR 307.1, 117.1a) by its card's own "you may cast this spell
    /// as though it had flash" (`CardDef::cast_as_though_flash`) - not by another permission its caster chose (CR 601.3), nor by
    /// an effect as it resolves: what Necromancy's "if you cast it any time a sorcery couldn't have been cast" asks
    /// (`CardDef::untimely_cleanup_rider`, its 2022-12-08 ruling). Kept only for a card whose text asks it.
    pub untimely_by_own_flash: bool,
    /// A spell paid for with mana that makes it uncounterable (CR 106.6, 701.5).
    pub cant_be_countered: bool,
    /// The number of the turn it was saddled (CR 702.171b): saddled until that turn ends. Zero for never.
    pub saddled_turn: u32,
    /// What was chosen for it - as it entered, or as an ability of it resolved - each with the ability that chose it
    /// (CR 607.2d, 607.2f): read only through `GameState::chosen_color` and its siblings, with the asking ability's link.
    pub choices: Choices,
    /// Who controlled it before the control-changing effects that apply to it now (CR 613.1b): it goes back to them
    /// when the last of those ends. `None` while no such effect applies.
    pub control_base: Option<PlayerId>,
    /// Phased out (CR 110.5, 702.26b), and how: what says when it phases in. `None` while it is phased in. The object is in
    /// `Zone::PHASED_OUT` exactly while this is set.
    pub phased: Option<Phased>,
    /// The face-down values of a face-down permanent (CR 708.2a; a cloaked one's ward {2}, CR 701.58a): the intrinsic card
    /// whose values its face-down status gives it in layer 1b, over its copy effects (CR 613.2b). Set as it enters face
    /// down, gone as it is turned face up.
    pub face_down_as: Option<CardId>,
}

/// The names an object keeps, chosen by different abilities of it as they resolved (Petrified Hamlet's, Medomai's
/// Prophecy's) or as it entered: one for each ability that chose one.
pub const CHOSEN_NAMES: usize = 2;

/// What was chosen for a permanent (CR 607.2d, 607.2f), each choice with the ability that made it (`Chosen`), its fields the
/// engine's alone (`ChoiceRecord`): card code reads a choice only through `GameState::chosen_color` and its siblings, with
/// the link of the ability that asks, so a permanent that has become a copy of something else, or an ability an effect
/// gave it, reads no choice that ability didn't make (CR 607.5).
///
/// The choices made as it entered are all made by one text, the text it entered with (`by`): its own "as this enters" is
/// the only one of its own that modifies how it enters (CR 614.12, 707.9a). A name may also be chosen as an ability of it
/// resolves (`names`), each ability's name kept apart; a later choice by the same ability replaces its earlier one. A
/// permanent that changes zones is a new object with none (CR 400.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Choices {
    /// The text that made the choices as it entered, as linked abilities know it (`Linked::text`). None while it chose
    /// nothing as it entered.
    by: Option<CardId>,
    color: Option<Color>,
    subtype: Option<Subtype>,
    player: Option<PlayerId>,
    option: Option<ChosenOption>,
    object: Option<ObjectId>,
    /// Card names, each with the ability that chose it: one chosen as it entered is its entering text's.
    names: [Option<Chosen<CardId>>; CHOSEN_NAMES],
}

/// What a permanent chose as it entered (CR 614.12a), for `ChoiceRecord::entered`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Entered {
    pub color: Option<Color>,
    pub subtype: Option<Subtype>,
    pub player: Option<PlayerId>,
    pub card: Option<CardId>,
    pub option: Option<ChosenOption>,
    pub object: Option<ObjectId>,
}

impl Entered {
    /// Whether anything was chosen.
    pub fn any(&self) -> bool {
        *self != Entered::default()
    }
}

/// The engine's own access to what a permanent chose (`Choices`): writing it, and reading it without asking which ability
/// reads. Not part of `cantrip_api`, so card code can't name it.
pub trait ChoiceRecord {
    /// The choices `text` made as the permanent entered (CR 614.12a), acquired as its own text.
    fn entered(text: CardId, made: Entered) -> Choices;
    /// `name` chosen by `by`, an ability of the permanent, as it resolved: replacing that ability's earlier name. Every
    /// ability's place taken (`CHOSEN_NAMES`), it is kept no more (beyond practical scale, docs/known-issues.md). Whether it
    /// was kept.
    fn name(&mut self, by: Linked, name: CardId) -> bool;
    /// What was chosen, with no ability asking: the scenario dump's and the loop watch's.
    fn entered_raw(&self) -> (Option<CardId>, Entered);
    /// The names, each with the ability that chose it.
    fn names_raw(&self) -> [Option<Chosen<CardId>>; CHOSEN_NAMES];
    /// The object chosen, renamed (the loop watch's canonical names).
    fn with_object(&self, object: Option<ObjectId>) -> Choices;
}

impl ChoiceRecord for Choices {
    fn entered(text: CardId, made: Entered) -> Choices {
        let own = |value| Chosen { value, text, acquired: Acquired::OWN };
        Choices {
            by: made.any().then_some(text),
            color: made.color,
            subtype: made.subtype,
            player: made.player,
            option: made.option,
            object: made.object,
            names: [made.card.map(own), None],
        }
    }

    fn name(&mut self, by: Linked, name: CardId) -> bool {
        let chosen = Chosen { value: name, text: by.text, acquired: by.acquired };
        let mine = |c: &Option<Chosen<CardId>>| c.is_some_and(|c| c.text == by.text && c.acquired == by.acquired);
        let slot = self.names.iter().position(mine).or_else(|| self.names.iter().position(Option::is_none));
        match slot {
            Some(at) => {
                self.names[at] = Some(chosen);
                true
            }
            None => false,
        }
    }

    fn entered_raw(&self) -> (Option<CardId>, Entered) {
        let card = self.names.iter().flatten().find(|c| Some(c.text) == self.by && c.acquired == Acquired::OWN).map(|c| c.value);
        (self.by, Entered { color: self.color, subtype: self.subtype, player: self.player, card, option: self.option, object: self.object })
    }

    fn names_raw(&self) -> [Option<Chosen<CardId>>; CHOSEN_NAMES] {
        self.names
    }

    fn with_object(&self, object: Option<ObjectId>) -> Choices {
        Choices { object, ..*self }
    }
}

impl Choices {
    /// The choices made as it entered, to `by`: only if `by` is an ability of the text that made them, which the permanent
    /// has as its own (CR 607.2d, 607.2f, 607.5).
    fn entered_to(&self, by: Linked) -> Option<&Choices> {
        (by.acquired == Acquired::OWN && self.by == Some(by.text)).then_some(self)
    }

    /// The color chosen, to `by` (`GameState::chosen_color`).
    pub fn color(&self, by: Linked) -> Option<Color> {
        self.entered_to(by)?.color
    }

    /// The creature or land type chosen, to `by`.
    pub fn subtype(&self, by: Linked) -> Option<Subtype> {
        self.entered_to(by)?.subtype
    }

    /// The player chosen, to `by`.
    pub fn player(&self, by: Linked) -> Option<PlayerId> {
        self.entered_to(by)?.player
    }

    /// The word chosen, to `by`.
    pub fn option(&self, by: Linked) -> Option<ChosenOption> {
        self.entered_to(by)?.option
    }

    /// The object chosen, to `by`, wherever it is now.
    pub fn object(&self, by: Linked) -> Option<ObjectId> {
        self.entered_to(by)?.object
    }

    /// The card name `by` chose (`Chosen::to`): as the permanent entered, or as `by` resolved.
    pub fn card(&self, by: Linked) -> Option<CardId> {
        self.names.iter().flatten().find_map(|c| c.to(by))
    }
}

/// One of the words a card's text offers to choose between, words that otherwise have no rules meaning (CR 607.2f): the
/// `index`th of `card`'s `labels` (the face whose text offered them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChosenOption {
    pub card: CardId,
    pub index: u8,
}

/// A choice kept on a permanent with the ability that made it, as linked abilities know it (CR 607.1, 607.5): the card
/// whose text it is printed in (`Linked::text`) and how the permanent acquired it - a card name chosen as an ability of it
/// resolved, which any ability it has may have made (`Marks::chosen_card`). "Choose a [value]" and "the chosen [value]" are
/// linked (CR 607.2d): the second refers only to a choice the first made, so only an ability of the same text, acquired the
/// same way, reads it (`to`). The choices made as it entered are kept with the one text that made them all
/// (`Marks::chosen_by`). A permanent that changes zones is a new object with none (CR 400.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chosen<T> {
    pub value: T,
    /// The card whose text made the choice, as linked abilities know it (`Linked::text`).
    pub text: CardId,
    /// How the permanent had the ability that made it (`Linked::acquired`).
    pub acquired: Acquired,
}

impl<T: Copy> Chosen<T> {
    /// The value chosen, to `by`, an ability of the permanent: only if `by` is linked to the ability that made the choice
    /// (CR 607.2d, 607.2f, 607.5) - of the same text, acquired the same way.
    pub fn to(&self, by: Linked) -> Option<T> {
        (by.text == self.text && by.acquired == self.acquired).then_some(self.value)
    }
}

/// How a permanent phased out (CR 702.26), which decides how it phases in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Phased {
    /// The characteristics just before simultaneous phase-out (CR 608.2h, 702.26b): source information while absent,
    /// not its projected current characteristics and not a zone-change/new-object snapshot.
    pub last_known: u32,
    /// Who controlled it as it phased out: it phases in during that player's untap step (CR 502.1), under their control.
    pub under: PlayerId,
    /// It phased out indirectly, with the permanent it is attached to (CR 702.26g), and phases in with that one alone.
    pub indirect: bool,
    /// It phased out "until [that permanent] leaves the battlefield" (CR 610.4): it phases in once that object has left,
    /// and not in an untap step (CR 610.4a).
    pub until_leaves: Option<ObjectId>,
    /// "Tap that creature as it phases in this way" (Oubliette): it phases in tapped by that second one-shot effect
    /// (CR 610.4), which is no tapping event.
    pub tapped_as_it_returns: bool,
}

/// How an object came to have an ability, as linked abilities know it (CR 607.1, 607.5): by its own text - what is printed
/// on it, or the copiable values it entered the battlefield with - or by an effect that gave it the ability later: a copy
/// effect (CR 707.2), named by the timestamp of the `EffectKind::RestoreCopy` it left, or an effect that granted the ability
/// (CR 613.1f), named by its timestamp and, for a static ability, which of its source's static abilities it is. Abilities
/// an object acquired by one effect are linked only to one another (CR 607.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Acquired {
    pub by: AcquiredBy,
    pub stamp: u32,
    pub part: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AcquiredBy {
    /// Its own text.
    #[default]
    Own,
    /// A copy effect applied to it after it entered.
    Copy,
    /// An effect that granted it the ability.
    Grant,
}

impl Acquired {
    pub const OWN: Acquired = Acquired { by: AcquiredBy::Own, stamp: 0, part: 0 };
}

/// An ability as linked abilities know it (CR 607.1, 607.2a, 607.5): the object it is an ability of, the card whose text it
/// is printed in - the object's own card, the card a copy effect gives it (CR 707.2), or the card whose ability an effect
/// granted (CR 607.1a) - read as a whole (either face of a transforming double-faced card, CR 607.1b, and either door of a
/// Room are that card: `CardTable::linked_card`), and how the object acquired it (`Acquired`). Two abilities of one object
/// are linked only when all three agree, so cards one exiled are not "exiled with" the object to an ability of another
/// text, nor to one a copy or grant effect gave it before or after (CR 607.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Linked {
    pub object: ObjectId,
    pub text: CardId,
    pub acquired: Acquired,
}

/// A card exiled by a permanent's ability, and remembered as exiled with it (CR 607.2a, 610.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExileLink {
    /// The permanent whose ability exiled it, as the object it was then.
    pub by: ObjectId,
    /// The card whose text that ability is printed in (`Linked::text`): only an ability of the same text refers to it (CR
    /// 607.2a, 607.5).
    pub text: CardId,
    /// How `by` acquired that ability (`Linked::acquired`): only an ability it acquired the same way refers to it (CR 607.5).
    pub acquired: Acquired,
    /// Where it was exiled from, and where it returns to.
    pub from: Zone,
    /// "Until [by] leaves the battlefield" (CR 610.3): it returns as soon as `by` is no longer there.
    pub until_leaves: bool,
    /// Exiled encoded on `by` by cipher (CR 702.99b): it is encoded on that creature while both stay where they are.
    pub encoded: bool,
}

impl ExileLink {
    /// Whether it was exiled by `ability` (CR 607.2a, 607.5): by that object, with an ability of that text it acquired so.
    pub fn is_by(&self, ability: Linked) -> bool {
        self.by == ability.object && self.text == ability.text && self.acquired == ability.acquired
    }

    /// A link to the ability `by`, from `from`.
    pub fn new(by: Linked, from: Zone, until_leaves: bool) -> ExileLink {
        ExileLink { by: by.object, text: by.text, acquired: by.acquired, from, until_leaves, encoded: false }
    }
}

impl Obj {
    /// Whether its record names another object: what it became, what it was prepared from, what it is attached to, what it
    /// entered with, what exiled it, what it is phased out until. Every field is named here, with no rest pattern, so a field
    /// added to `Obj` or `Marks` must be sorted into a link or not before the crate compiles (the announcement search in
    /// `cantrip-rules` relies on it to know a card's record tells it of no other object).
    pub fn links_an_object(&self) -> bool {
        let Obj {
            card: _,
            owner: _,
            controller: _,
            zone: _,
            token: _,
            tapped: _,
            controlled_since: _,
            damage: _,
            deathtouched: _,
            counters: _,
            timestamp: _,
            copy: _,
            loyalty_turn: _,
            became,
            last_known: _,
            exile_on_leave: _,
            kicked: _,
            paid: _,
            gift: _,
            spent: _,
            cast_way: _,
            class_level: _,
            prepared: _,
            card_copy: _,
            one_face: _,
            prepared_from,
            on_adventure: _,
            cast_from: _,
            activated_turn: _,
            activated_mask: _,
            attached_to,
            attached_player: _,
            entered_with,
            x: _,
            marks,
            transformed: _,
            exiled,
            face_down: _,
            seen_by: _,
        } = *self;
        let Marks {
            doors: _,
            exerted_by: _,
            monstrous: _,
            solved: _,
            protector: _,
            gains_haste: _,
            gains: _,
            untimely_by_own_flash: _,
            cant_be_countered: _,
            saddled_turn: _,
            choices,
            control_base: _,
            phased,
            face_down_as: _,
        } = marks;
        became.is_some()
            || prepared_from.is_some()
            || attached_to.is_some()
            || entered_with.is_some()
            || exiled.is_some()
            || phased.is_some_and(|p| p.until_leaves.is_some())
            || choices.entered_raw().1.object.is_some()
    }

    /// A fresh object in `zone`, with nothing about it set: the rules' own source for an ability no card has (the
    /// monarch's, a dungeon room's), which is in no zone's list.
    pub fn new(card: CardId, owner: PlayerId, zone: Zone) -> Obj {
        Obj {
            card,
            owner,
            controller: owner,
            zone,
            token: false,
            tapped: false,
            controlled_since: 0,
            damage: 0,
            deathtouched: false,
            counters: [0; CounterKind::COUNT],
            timestamp: 0,
            copy: None,
            loyalty_turn: 0,
            became: None,
            last_known: None,
            exile_on_leave: false,
            kicked: 0,
            paid: 0,
            gift: None,
            spent: ManaTally::default(),
            cast_way: 0,
            class_level: 0,
            prepared: false,
            card_copy: false,
            one_face: false,
            prepared_from: None,
            on_adventure: None,
            cast_from: None,
            activated_turn: 0,
            activated_mask: 0,
            attached_to: None,
            attached_player: None,
            entered_with: None,
            x: 0,
            marks: Marks::default(),
            transformed: false,
            exiled: None,
            face_down: false,
            seen_by: 0,
        }
    }

    pub fn counters(&self, kind: CounterKind) -> i16 {
        self.counters[kind as usize]
    }

    /// A face-down card in a zone other than the battlefield and the stack (a card exiled face down): it has no
    /// characteristics (CR 406.3a). A face-down permanent, phased out or not, and a face-down spell have those of CR 708.2a.
    pub fn face_down_card(&self) -> bool {
        self.face_down && !matches!(self.zone.kind(), ZoneKind::Battlefield | ZoneKind::PhasedOut | ZoneKind::Stack)
    }

    /// Its copiable values (CR 707.2) where they are not its card's own: its copy effects' (`copy`, layer 1a), and over
    /// them, while it is a face-down permanent, the face-down values (layer 1b, CR 613.2b, 708.2a) - so a face-down
    /// permanent that becomes a copy is still a nameless 2/2, and the copy once turned face up (CR 708.10). What every
    /// reader of what an object is reads; `copy` alone is what it is once turned face up.
    pub fn copiable(&self) -> Option<Copiable> {
        match self.face_down_values() {
            Some(card) => Some(Copiable::of(card)),
            None => self.copy,
        }
    }

    /// The face-down values a face-down permanent has in layer 1b (`Marks::face_down_as`): set only as a permanent enters
    /// face down and cleared as it is turned face up, and a new object starts with none, so a card exiled face down
    /// (which has no characteristics at all) never has them.
    pub fn face_down_values(&self) -> Option<CardId> {
        self.marks.face_down_as
    }

    /// The card whose values it has from its copy effects and face-down status (layers 1a and 1b), if not its own card:
    /// `copiable().map(|c| c.card)` without copying the values.
    ///
    /// A double-faced token's own copiable values (CR 707.8a) are no copy effect over another card: its faces are those of
    /// its card, as a double-faced card's are, and its exceptions apply to whichever is up (`layers`), so it has none here.
    pub fn copied_card(&self) -> Option<CardId> {
        match self.marks.face_down_as {
            Some(card) => Some(card),
            None => self.copy.as_ref().filter(|c| !self.own_faces(c)).map(|c| c.card),
        }
    }

    /// Whether `copy`, its copy values, are a double-faced token's own faces (CR 707.8a): a token whose card is the front
    /// face of the double-faced card or token it copies, and whose values its creation kept for their exceptions, which
    /// apply to whichever face is up (`Copiable::own_faces`). Values a copy effect gives it later - Mirrorweave, Saheeli's
    /// "becomes a copy", a Clone's copy it entered as - are a copy effect over it, as on a card (CR 707.2).
    pub fn own_faces(&self, copy: &Copiable) -> bool {
        self.token && copy.is_own_faces()
    }

    /// Whether it is represented by a double-faced card or a double-faced token (CR 701.27a, 707.8a, 712.9), given whether
    /// its card has a back face: a card with a copy effect on it is still its card (Glasspool Mimic); a token made as a copy
    /// of one face alone (a copy of a Clone that copied a double-faced permanent, CR 707.8a's example) is single-faced.
    pub fn represented_double_faced(&self, card_has_back: bool) -> bool {
        card_has_back && !(self.token && self.one_face)
    }

    /// Whether it has any counter on it.
    pub fn has_counters(&self) -> bool {
        self.counters.iter().any(|&c| c != 0)
    }

    /// Every kind of counter on it, by `CounterKind` number.
    pub fn counter_array(&self) -> [i16; CounterKind::COUNT] {
        self.counters
    }

    /// Takes up to `n` counters of one kind off it (CR 122.1), never below none. Taking counters off is not putting
    /// them on, so anything may do it.
    pub fn remove_counters(&mut self, kind: CounterKind, n: i16) {
        let c = &mut self.counters[kind as usize];
        *c = (*c - n.max(0)).max(0);
    }

    /// Takes every counter off it: an object made into a normal form, for comparing positions (`cantrip_rules::loops`).
    pub fn clear_counters(&mut self) {
        self.counters = [0; CounterKind::COUNT];
    }

    /// The write of the put path. Only `cantrip_rules::counters` calls this, once it has named the player putting the
    /// counters and applied the replacement effects of their being put (CR 122.6, 122.6a, 614.1a); a test there,
    /// `only_the_put_path_puts_counters`, fails the build of any other caller. Negative counts are ignored.
    #[doc(hidden)]
    pub fn put_path_add(&mut self, counts: &[i16; CounterKind::COUNT]) {
        for (n, &add) in self.counters.iter_mut().zip(counts) {
            if add > 0 {
                *n = n.saturating_add(add);
            }
        }
    }
}

/// An ability an exception of a copy effect adds (CR 707.9b), and how many instances of it the copy has (CR 113.2c). An
/// empty slot has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtraAbility {
    pub card: CardId,
    pub index: u8,
    pub instances: u8,
}

impl ExtraAbility {
    pub const NONE: ExtraAbility = ExtraAbility { card: CardId(0), index: 0, instances: 0 };

    pub fn ability(&self) -> AbilityRef {
        AbilityRef { card: self.card, index: self.index }
    }
}

/// Copiable values (CR 707.2): a card's printed values, and the exceptions copy effects have added to them
/// (CR 707.9b-d). A copy of a copy copies these, exceptions included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Copiable {
    pub card: CardId,
    /// Card types added by exceptions: Phyrexian Metamorph is an artifact in addition to its other types. With
    /// its types stated (`types_stated`), every card type it has but a kept instant or sorcery (CR 205.1a).
    pub extra_types: TypeSet,
    /// Subtypes added by exceptions, as Phantasmal Image is an Illusion in addition to its other types. With
    /// its types stated, every subtype it has.
    pub extra_subtypes: SubtypeSet,
    /// Abilities added by exceptions, in order, each distinct ability once with the number of its instances (CR 113.2c):
    /// a Thespian's Stage that copies a Stage-copy, or itself, has one more instance of the Stage's ability each time.
    pub extra_abilities: [ExtraAbility; Copiable::EXTRA_ABILITIES],
    /// Keywords added by exceptions: "except it has haste".
    pub extra_keywords: Keywords,
    /// "Except it's black" (CR 707.9b): these are its colors, and a color-defining ability it copies is left behind
    /// (CR 707.9d).
    pub colors: Option<ColorSet>,
    /// "Except it's 4/4" (CR 707.9b, 707.9d): its power and toughness, whatever the copied card has or defines, when
    /// `stated` has `STATED_PT` (`pt`), and 0/0 otherwise.
    pub stated_pt: (i16, i16),
    /// "Except its name is X" (CR 707.9b): its name, whatever the copied card is called.
    pub name: Option<&'static str>,
    /// The exceptions that are a yes or no, one bit each (`Copiable::STATED_*`): read and set through `types_stated`,
    /// `has_no_cost`, `legendary_stated`, `stated_loyalty`, `pt`, `faces` and their builders. Packed, with `loyalty`,
    /// `stated_pt` and `front`, into the bytes the values had to spare, so an `Obj` pays nothing for the rare exceptions.
    pub stated: u16,
    /// "Except it has starting loyalty X": the loyalty, when `stated` has `STATED_LOYALTY`, and 0 otherwise.
    pub loyalty: i16,
    /// The words of its text changed from the card's (CR 612.4, 612.2a): a token made by an ability whose text was
    /// changed has the colors and creature types the changed text names, and so does a copy of it.
    pub words: Words,
    /// No exception: with `STATED_FACES_OF`, the front face of the double-faced card or token these are the copiable
    /// values of (CR 707.8a), with `STATED_BACK_UP` if its back face was up - so a token made as a copy of them is
    /// double-faced, its card that front face (`faces`). A copy effect on an object copies `card` alone, whatever this says
    /// (CR 707.8: the face that is up). Kept in the bytes the values had to spare, as `stated` is.
    pub front: CardId,
}

impl Copiable {
    /// A card's own printed values: a copy of it with no exceptions.
    pub const fn of(card: CardId) -> Copiable {
        Copiable {
            card,
            extra_types: TypeSet::NONE,
            extra_subtypes: SubtypeSet::NONE,
            extra_abilities: [ExtraAbility::NONE; Copiable::EXTRA_ABILITIES],
            extra_keywords: Keywords::NONE,
            colors: None,
            stated_pt: (0, 0),
            name: None,
            stated: 0,
            loyalty: 0,
            words: Words::NONE,
            front: CardId(0),
        }
    }

    /// "Except it's a 5/5 black Demon" (CR 707.9b, 205.1a): the exception states what it is (`typed`).
    pub const STATED_TYPES: u16 = 1;
    /// "Except it has no mana cost" (`costless`).
    pub const STATED_NO_COST: u16 = 1 << 1;
    /// "Except it's legendary" or "except it isn't legendary" (`legendary`, `nonlegendary`); which is `LEGENDARY`.
    pub const STATED_LEGENDARY: u16 = 1 << 2;
    pub const LEGENDARY: u16 = 1 << 3;
    /// "Except it has starting loyalty X" (`with_loyalty`).
    pub const STATED_LOYALTY: u16 = 1 << 4;
    /// No exception: with `faces`, the double-faced object these are the values of had its back face up (CR 707.8a: a
    /// token copy of it enters back face up).
    pub const STATED_BACK_UP: u16 = 1 << 5;
    /// No exception: a double-faced token's own copy values, kept by its creation for their exceptions over its faces
    /// (CR 707.8a, `Obj::own_faces`). Never on values a copy effect gives an object.
    pub const STATED_OWN_FACES: u16 = 1 << 6;
    /// No exception: `front` names the double-faced object these are the values of (`faces`).
    pub const STATED_FACES_OF: u16 = 1 << 7;
    /// "Except it's [power]/[toughness]" (`sized`): `stated_pt` is its power and toughness.
    pub const STATED_PT: u16 = 1 << 8;
    /// The bits that are no exception.
    const STATED_FACES: u16 = Copiable::STATED_BACK_UP | Copiable::STATED_OWN_FACES | Copiable::STATED_FACES_OF;

    /// The front face of the double-faced card or token these are the copiable values of (CR 707.8a), if they are one's.
    pub fn faces(&self) -> Option<CardId> {
        (self.stated & Copiable::STATED_FACES_OF != 0).then_some(self.front)
    }

    /// These values, as those of the double-faced card or token whose front face is `front`, back face up if `back_up`
    /// (`faces`), or of no double-faced object with `None`.
    pub fn from_faces(mut self, front: Option<CardId>, back_up: bool) -> Copiable {
        self.stated &= !Copiable::STATED_FACES;
        self.front = front.unwrap_or(CardId(0));
        if front.is_some() {
            self.stated |= Copiable::STATED_FACES_OF;
            if back_up {
                self.stated |= Copiable::STATED_BACK_UP;
            }
        }
        self
    }

    /// Whether the double-faced object these are the values of (`faces`) had its back face up.
    pub fn back_up(&self) -> bool {
        self.stated & Copiable::STATED_BACK_UP != 0
    }

    /// These values as a double-faced token's own, over its faces from `front` (`STATED_OWN_FACES`): only a token's
    /// creation makes them.
    pub fn own_faces_of(self, front: CardId) -> Copiable {
        let mut c = self.from_faces(Some(front), false);
        c.stated |= Copiable::STATED_OWN_FACES;
        c
    }

    /// Whether these are a double-faced token's own copy values (`STATED_OWN_FACES`).
    pub fn is_own_faces(&self) -> bool {
        self.stated & Copiable::STATED_OWN_FACES != 0
    }

    /// "Except it's a 5/5 black Demon" (CR 707.9b, 205.1a): the exception states what it is, so its card types and
    /// subtypes are `extra_types` and `extra_subtypes` instead of the copied card's, and a characteristic-defining
    /// ability that defines them (Changeling) is not copied (CR 707.9d). See `Copiable::typed`.
    pub fn types_stated(&self) -> bool {
        self.stated & Copiable::STATED_TYPES != 0
    }

    /// "Except it has no mana cost" (CR 707.9b): its mana value is 0 and it cannot be cast for a cost.
    pub fn has_no_cost(&self) -> bool {
        self.stated & Copiable::STATED_NO_COST != 0
    }

    /// "Except it's legendary" (`Some(true)`) or "except it isn't legendary" (`Some(false)`, CR 707.9b, 205.4): whether it
    /// has the legendary supertype, whatever the copied card has; its other supertypes are the card's. `None` keeps the
    /// card's. A later exception overrides an earlier one: a copy of a copy is what the copy is, then this copy's exceptions
    /// (CR 707.9b).
    pub fn legendary_stated(&self) -> Option<bool> {
        (self.stated & Copiable::STATED_LEGENDARY != 0).then_some(self.stated & Copiable::LEGENDARY != 0)
    }

    /// "Except it has starting loyalty X" (CR 707.9b, 306.5b): its printed loyalty, which a planeswalker enters with as
    /// loyalty counters, whatever the copied card prints. Ob Nixilis, the Adversary's copy.
    pub fn stated_loyalty(&self) -> Option<i16> {
        (self.stated & Copiable::STATED_LOYALTY != 0).then_some(self.loyalty)
    }

    /// "Except its name is X."
    pub fn named(mut self, name: &'static str) -> Copiable {
        self.name = Some(name);
        self
    }

    /// Whether any exception is stated: a copy with none is the card itself.
    pub fn has_exceptions(&self) -> bool {
        self.extra_types.0 != 0
            || !self.extra_subtypes.is_empty()
            || self.stated & !Copiable::STATED_FACES != 0
            || self.extra_abilities[0].instances > 0
            || self.extra_keywords.0 != 0
            || self.colors.is_some()
            || self.pt().is_some()
            || self.name.is_some()
            || !self.words.is_printed()
    }

    /// "Except it's a [type] in addition to its other types."
    pub fn with_type(mut self, t: CardType) -> Copiable {
        self.extra_types = self.extra_types.with(t);
        self
    }

    /// "Except it's a [subtypes] [types]", with no "in addition to its other types" (CR 707.9b, 205.1a): these are its card
    /// types and subtypes instead of the copied card's, as the rulings spell out ("The token is a Demon creature instead of
    /// its other types and is black instead of its other colors"). Supertypes are kept: a copy of a legendary card is
    /// legendary. An instant or sorcery keeps that type (CR 205.1a). A characteristic-defining ability of the copied card
    /// that defines its subtypes is not copied (CR 707.9d: Changeling). It replaces the types and subtypes earlier
    /// exceptions of these values added, and an exception applied after it adds to it; a copy of the copy is what the copy
    /// is (CR 707.9b). Ardyn, the Usurper ("except it's a 5/5 black Demon": `.typed(TypeSet::of(&[CardType::Creature]),
    /// SubtypeSet::of(&[Subtype::Demon])).colored(ColorSet::of(Color::Black)).sized(5, 5)`), The Scarab God, God-Pharaoh's
    /// Gift and Hour of Eternity ("except it's a 4/4 black Zombie") print it. Eternalize and embalm (CR 702.128a, 702.129a)
    /// make a Zombie in addition to its other types, and use `with_subtype`.
    pub fn typed(mut self, types: TypeSet, subtypes: SubtypeSet) -> Copiable {
        self.extra_types = types;
        self.extra_subtypes = subtypes;
        self.stated |= Copiable::STATED_TYPES;
        self
    }

    /// "Except it's a [subtype] in addition to its other types."
    pub fn with_subtype(mut self, s: Subtype) -> Copiable {
        self.extra_subtypes = self.extra_subtypes.with(s);
        self
    }

    /// The most distinct abilities exceptions add to one copy. Each card that prints such an exception adds its own one
    /// ability (Thespian's Stage, Phantasmal Image, Evil Twin, Likeness Looter), so a copy gathers one per such card in its
    /// chain of copies. A list that is full takes no more, as every full list does.
    pub const EXTRA_ABILITIES: usize = 6;

    /// "Except it has [this ability]." Another instance of an ability the exceptions already add is counted in its slot
    /// (CR 113.2c), up to 255 of them, which is past what any object holds (`MAX_ABILITIES`). A new one with every slot
    /// taken (`EXTRA_ABILITIES`) is not added: the values are left as they were.
    pub fn with_ability(mut self, ability: AbilityRef) -> Copiable {
        let slot = self
            .extra_abilities
            .iter()
            .position(|e| e.instances > 0 && e.ability() == ability)
            .or_else(|| self.extra_abilities.iter().position(|e| e.instances == 0));
        let Some(slot) = slot else { return self };
        let e = &mut self.extra_abilities[slot];
        *e = ExtraAbility { card: ability.card, index: ability.index, instances: e.instances.saturating_add(1) };
        self
    }

    /// Every instance of the abilities the exceptions add, in order (CR 707.9b, 113.2c).
    pub fn extra_instances(&self) -> impl Iterator<Item = AbilityRef> + '_ {
        self.extra_abilities.iter().take_while(|e| e.instances > 0).flat_map(|e| std::iter::repeat_n(e.ability(), usize::from(e.instances)))
    }

    /// Whether an exception adds `ability`.
    pub fn adds_ability(&self, ability: AbilityRef) -> bool {
        self.extra_abilities.iter().any(|e| e.instances > 0 && e.ability() == ability)
    }

    /// "Except it has [keyword]."
    pub fn with_keyword(mut self, k: Keyword) -> Copiable {
        self.extra_keywords = self.extra_keywords.with(k);
        self
    }

    /// "Except it's [color]."
    pub fn colored(mut self, colors: ColorSet) -> Copiable {
        self.colors = Some(colors);
        self
    }

    /// "Except it's [power]/[toughness]."
    pub fn sized(mut self, power: i16, toughness: i16) -> Copiable {
        self.stated |= Copiable::STATED_PT;
        self.stated_pt = (power, toughness);
        self
    }

    /// "Except it's [power]/[toughness]" (`sized`), if it says so.
    pub fn pt(&self) -> Option<(i16, i16)> {
        (self.stated & Copiable::STATED_PT != 0).then_some(self.stated_pt)
    }

    /// "Except it has no mana cost."
    pub fn costless(mut self) -> Copiable {
        self.stated |= Copiable::STATED_NO_COST;
        self
    }

    /// "Except it's legendary" (CR 707.9b, 205.4): the supertype added to whatever the copied card has.
    pub fn legendary(mut self) -> Copiable {
        self.stated |= Copiable::STATED_LEGENDARY | Copiable::LEGENDARY;
        self
    }

    /// "Except it isn't legendary" (CR 707.9b, 205.4): the supertype taken from whatever the copied card has, its other
    /// supertypes kept.
    pub fn nonlegendary(mut self) -> Copiable {
        self.stated = (self.stated | Copiable::STATED_LEGENDARY) & !Copiable::LEGENDARY;
        self
    }

    /// "Except it has starting loyalty N" (CR 707.9b, 306.5b): the loyalty it is printed with.
    pub fn with_loyalty(mut self, loyalty: i16) -> Copiable {
        self.stated |= Copiable::STATED_LOYALTY;
        self.loyalty = loyalty;
        self
    }
}

/// What a target is (CR 115).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Target {
    #[default]
    None,
    Player(PlayerId),
    Object(ObjectId),
    /// A triggered or activated ability on the stack (CR 115.1, 113.7a), named by the serial it was put there with.
    Ability(u32),
}

impl Target {
    pub fn object(self) -> Option<ObjectId> {
        match self {
            Target::Object(id) => Some(id),
            _ => None,
        }
    }

    pub fn player(self) -> Option<PlayerId> {
        match self {
            Target::Player(p) => Some(p),
            _ => None,
        }
    }
}

/// How many targets a stack item holds in itself; past that, all of them are held in `GameState`'s journaled target arena
/// (`GameState::spill_targets`), where the item says they start, so an ordinary item stays small to copy and the rules set
/// no limit (CR 115.1d: "any number of target ...", Appa; X targets).
pub const INLINE_TARGETS: usize = 16;


/// What kind of thing is on the stack (CR 405).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackKind {
    Spell,
    Activated,
    Triggered,
}

/// One spell or ability on the stack, with everything announced for it (CR 601.2, 602.2, 603.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StackItem {
    pub kind: StackKind,
    /// The spell's own object, or the ability's source as it was when the ability was put on the stack.
    pub object: ObjectId,
    /// The ability: for a spell, its card's spell ability.
    pub ability: AbilityRef,
    pub controller: PlayerId,
    /// The first `INLINE_TARGETS` targets: read them through `GameState::targets` (or `target` for one of the first), and write
    /// them through `GameState::set_item_targets`, which keeps the spill of a longer list.
    pub inline_targets: [Target; INLINE_TARGETS],
    pub target_count: u32,
    /// For each target, parallel to `targets`, the target slot it was chosen for (CR 601.2c): the index of its spec in the
    /// spell's or ability's printed list (a modal item's: in the list of its chosen modes' specs, in printed order, CR
    /// 700.2c). So a slot that takes a variable number of targets ("attach any number of target Equipment you control to
    /// target creature you control") can stand anywhere in the list, and each target is judged by its own slot.
    pub inline_spec_of: [u8; INLINE_TARGETS],
    /// The X announced for it (CR 107.3); for a reflexive triggered ability, the number its "that many" counts
    /// (CR 603.12, `PendingTrigger::count`).
    pub x: u16,
    /// How many times it was kicked (CR 702.33).
    pub kicked: u8,
    /// What its target slots' `TargetSpec::COUNTED` read as they were chosen (CR 601.2c, 603.3d): a spell's X, or else the
    /// times it was kicked; an activated ability's X; a triggered ability's count it was given (`PendingTrigger::count`), or
    /// else its source's X or times kicked. Read again as its targets are judged together anew (`TargetSpec::together`, CR
    /// 115.7, 608.2b).
    pub counted: u16,
    /// How many times its `optional_cost` was paid (CR 601.2b, 707.10: a copy has the same).
    pub paid: u32,
    /// Which way it was cast (0 for the mana cost, n for the nth alternative cost), or activated.
    pub way: u8,
    /// The modes chosen as it was put on the stack (CR 700.2): a bit for each mode of its modal definition.
    pub modes: u32,
    /// Card code's own values chosen as it was put on the stack: a mode, a pile, a number.
    pub choice: u32,
    /// What the event that triggered it was about (CR 603.2): not a target. For an ability or a permanent spell whose cost
    /// returned an attacking creature to hand (ninjutsu, sneak: CR 702.49a, 702.190b), what that creature was attacking.
    pub subject: Target,
    /// What the event that triggered it carried (CR 603.2): the spell cast, the object that targeted.
    pub data: [u32; 2],
    /// The objects it is about that are not its targets, in `GameState::members`. For a "one or more" ability (CR 603.2c):
    /// the objects of the batch that triggered it. For an activated ability: the permanents its cost chose and tapped,
    /// sacrificed or returned (CR 601.2h by way of 602.2b), which its effect may name ("the tapped creature's power",
    /// station, CR 702.184a); a copy of the ability has the same (CR 707.10). For a spell or an activated ability, after
    /// those, the cards its cost or announcement revealed from a hand (behold, CR 701.4a; splice, CR 702.47a), revealed until
    /// it leaves the stack (CR 701.20a).
    pub members: Span,
    /// What names this item while it is on the stack, so that a spell or ability can target it (CR 115.1).
    pub serial: u32,
    /// For a spell cast with gift promised (CR 702.174): the opponent it was promised to.
    pub gift: Option<PlayerId>,
    /// For a spell or ability that divides a total among its targets (CR 601.2d): where each target's share, parallel to
    /// `targets`, starts in the game's share arena (`GameState::shares`); `StackItem::NO_SHARES` for an item with no share
    /// above 0, which is every other item.
    pub shares: u32,
    /// For an item with more than `INLINE_TARGETS` targets, where all `target_count` of them and their slots start in the
    /// game's target arena (`GameState::targets`); 0 otherwise.
    pub spill: usize,
    /// The "if that spell would be put into your graveyard, exile it instead" of the effect that had it cast (CR 614.1a):
    /// a replacement of its move to its owner's graveyard only, ordered with the others that would change that move (CR
    /// 616.1; `ExileInstead`).
    pub exile_instead: ExileInstead,
    /// For a spell cast with flashback that is not printed in its ways (CR 702.34a): one a permission gave (Snapcaster Mage)
    /// or a permanent's static ability (Iroh, Grand Lotus), or an aftermath half cast from a graveyard (CR 702.127a). It is
    /// exiled instead of put anywhere else any time it would leave the stack.
    pub flashback: bool,
    /// The serial of the first event in the history after it was put on the stack (the history's head then), or, for a
    /// delayed triggered ability, after that ability was created (`Delayed::since`): what an instruction to transform its
    /// source asks the history about (CR 701.27f).
    pub since: u32,
    /// A copy of an ability (CR 707.10): it was neither activated nor triggered, so it is not the instance of a state
    /// trigger that has triggered and not yet left the stack (CR 603.8). A copy of a spell is told by its object
    /// (`Obj::card_copy`).
    pub copied: bool,
    /// For a spell that was cast: the row of `GameState::last_known` that keeps it as it became cast (CR 601.2i), which
    /// `history::Cast::chars` reads. `u32::MAX` for anything else, a copy of a spell included (CR 707.10).
    pub cast_known: u32,
    /// How its source acquired the ability it is (CR 607.5): what an ability linked to it is linked by (`Query::linked`).
    /// Own for a spell.
    pub acquired: Acquired,
}

impl StackItem {
    /// No shares: every target's is 0.
    pub const NO_SHARES: u32 = u32::MAX;

    /// Target `slot` among the first `INLINE_TARGETS`, which the item holds itself, or `Target::None` past the targets
    /// announced. A later one is read through `GameState::target`.
    pub fn target(&self, slot: usize) -> Target {
        assert!(slot < INLINE_TARGETS || self.holds_its_targets(), "target {slot} of a stack item is read through the game state");
        if slot < self.target_count as usize { self.inline_targets[slot] } else { Target::None }
    }

    /// Whether its targets are all in the item itself, so `inline` is the whole list.
    pub fn holds_its_targets(&self) -> bool {
        self.target_count as usize <= INLINE_TARGETS
    }
}

/// A continuous effect from a resolved spell or ability (CR 611), applied in its layer (CR 613).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContinuousEffect {
    pub source: ObjectId,
    pub controller: PlayerId,
    pub timestamp: u32,
    pub affected: ObjectId,
    pub kind: EffectKind,
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// Protection from these colors (CR 702.16), layer 6.
    Protection(ColorSet),
    /// Protection from colorless (CR 702.16a, 105.2c), layer 6.
    ProtectionFromColorless,
    /// Protection from objects of these card types (CR 702.16a): "protection from creatures", layer 6.
    ProtectionFromTypes(TypeSet),
    /// Protection from everything (CR 702.16e), layer 6.
    ProtectionFromEverything,
    /// +N/+M (CR 613.4c), layer 7c.
    Pump(i16, i16),
    /// Gains these keywords, layer 6.
    AddKeywords(Keywords),
    /// Loses these keywords (CR 613.1f), layer 6, at the effect's timestamp: "permanents your opponents control lose hexproof
    /// and indestructible" (Shadowspear). What is granted after it is kept.
    LoseKeywords(Keywords),
    /// Gains this ability (a triggered or activated ability granted to it), layer 6.
    AddAbility(AbilityRef),
    /// Protection from a player (CR 702.16a): their spells and abilities can't target it, their creatures can't block
    /// it, and damage their sources would deal to it is prevented. Layer 6.
    ProtectionFromPlayer(PlayerId),
    /// Gains these card types (CR 205.1b, 613.1d), layer 4: a crewed Vehicle is an artifact creature, and Tezzeret's
    /// emblem makes an artifact a Robot creature.
    AddTypes(TypeSet),
    /// Its controller is this player (CR 613.1b, layer 2). The latest of an object's control effects applies.
    Control(PlayerId),
    /// Gains "this creature attacks each combat if able" (CR 508.1d), layer 6: an ability, which "loses all abilities" later
    /// takes (Carnage, Crimson Chaos).
    MustAttack,
    /// Gains toxic N (CR 702.164a), layer 6: an instance of its own, added to the ones it has (CR 702.164b), which "loses
    /// all abilities" later takes.
    Toxic(u16),
    /// "Attacks [that player or permanent] if able" (CR 508.1d): an attack requirement that names what to attack, and only
    /// that - Ursine Monstrosity's "attacks that player this combat if able", Gideon, Battle-Forged's "attacks Gideon during
    /// its controller's next turn if able". Where the creature can't attack that one - a cost to pay, a planeswalker gone or
    /// that can't be attacked - it may attack anything else, or nothing (the rulings of both). A permanent is named as the
    /// object it is: one that left and came back is a new object it never names (CR 400.7). An effect, not an ability: a
    /// creature that loses its abilities is still under it.
    MustAttackAt(Target),
    /// It becomes this subtype in addition to its other subtypes (CR 613.1d), layer 4.
    AddSubtype(Subtype),
    /// Its creature type becomes this one (CR 205.1a, 613.1d), layer 4: every creature type it had is replaced, its card types
    /// and its other subtypes (artifact types, land types) stay: "becomes a green Wurm" (Scale Up).
    SetCreatureType(Subtype),
    /// It gains all creature types (CR 205.3m, 613.1d), layer 4, in addition to its other subtypes: "becomes a 2/2
    /// creature with all creature types" (Mutavault, Soulstone Sanctuary). Only a creature or a Kindred can have creature
    /// types (CR 205.3d), so it gives them to an object that is one by its timestamp. It is not Changeling: the object
    /// gains no ability (the Mutavault ruling).
    AddAllCreatureTypes,
    /// It loses all creature types (CR 205.3m, 613.1d), layer 4, at its timestamp: its other subtypes and its card types stay,
    /// and a creature type an effect gives it later is had (the Nameless Inversion rulings). Changeling's types are taken
    /// too, the ability having applied first (CR 613.3); the ability itself stays.
    LoseAllCreatureTypes,
    /// Its power and toughness are switched (CR 613.4d), layer 7d: applied after every setting and modifying effect and
    /// the counters.
    SwitchPt,
    /// Its power and toughness become these (CR 613.4b), layer 7b: "it becomes a 0/0 creature".
    SetPt(i16, i16),
    /// Is exactly these card types (CR 205.1a, 613.1d), layer 4: an instant or sorcery stays one, and the subtypes it had stay
    /// only where they belong to a card type it still has (`Subtype::card_types`). An effect that also names its subtypes
    /// ("becomes an Equipment artifact") sets them with `LoseSubtypesOf` and `AddSubtype` after it.
    SetTypes(TypeSet),
    /// It loses every subtype of this card type's set (CR 205.1a, 613.1d), layer 4, at its timestamp: what an effect that
    /// sets its subtypes of that kind does before adding them - "becomes an Equipment artifact" (The Irencrag), "It's an
    /// Aura enchantment" (`EntersAs::aura`). Its card types stay. Creature types are `LoseAllCreatureTypes`.
    LoseSubtypesOf(CardType),
    /// Is exactly these colors (CR 105.3, 613.1e), layer 5.
    SetColors(ColorSet),
    /// "Loses all abilities" (CR 613.1f), layer 6: at its timestamp, so what is granted later is kept.
    LoseAllAbilities,
    /// It loses the enchant ability `card` prints (CR 113.10, 613.1f, 702.5a), layer 6, at its timestamp: every instance of
    /// it (CR 113.10b), whether its own card's or a copy effect's (`AbilityRef::enchant`); one given after it is kept. Animate
    /// Dead's and Dance of the Dead's "it loses 'enchant creature card in a graveyard'", as their enters ability resolves.
    /// An enchant ability is read only off the object's abilities (`attach::enchants`), so taking it from them is all
    /// the loss is.
    LoseEnchant(CardId),
    /// Goaded by the effect's controller (CR 701.15a): attacks each combat if able, and attacks a player other than
    /// the goader if able (CR 701.15b). An effect, not an ability: a goaded creature that loses its abilities is still goaded.
    Goaded,
    /// "Can't block" (CR 509.1b) for as long as the effect lasts.
    CantBlock,
    /// "It can't be blocked this turn" (CR 509.1b) for as long as the effect lasts: Escape Tunnel's, Manifold Key's, Creeping
    /// Tar Pit's. A resolved spell or ability that says so without "has" or "gains" makes an effect on the creature, not an
    /// ability it gains (CR 113.1a, 611.2c), so a creature that loses its abilities afterwards (CR 613.1f: Turn to Frog) still
    /// can't be blocked. "Gains 'this creature can't be blocked'" and a printed "can't be blocked" are
    /// `Keyword::CantBeBlocked`, which losing its abilities takes.
    CantBeBlocked,
    /// A restriction on blocking (CR 509.1b) a resolved spell or ability made, for as long as the effect lasts: the `index`th
    /// of the `lasting_block_rules` of `card`, the card whose spell or ability made it, which says which blocks it forbids.
    /// Gingerbrute's "this creature can't be blocked this turn except by creatures with haste". An effect, not an ability
    /// (CR 113.1a, 611.2c): it holds whatever abilities its source or the creature loses afterwards (CR 613.1f), and it ends
    /// with its `affected` object (CR 400.7).
    BlockRestriction { card: CardId, index: u8 },
    /// "Blocks this turn if able" (CR 509.1c): a requirement that the creature block, any attacker it is able to, in each
    /// declaration of blockers while the effect lasts - with `Duration::EndOfTurn`, each declare blockers step of the turn
    /// (CR 509.1c's last sentence). One requirement however many creatures it could block. It is an effect, not an ability
    /// (CR 113.1): a creature that loses its abilities still blocks if able. Obeyed only as far as the restrictions allow and
    /// at no cost: a tapped creature, one that can't block, or one with no attacker it may block is not required to (the
    /// rulings).
    MustBlock,
    /// "Blocks [this creature] this combat if able" (CR 509.1c, 702.39a): a requirement that the creature block this attacker,
    /// and only that one, in each declaration of blockers while the effect lasts - `Duration::EndOfCombat` for provoke. Where
    /// it may not block that attacker - tapped, can't block, restricted, or the attacker no longer attacking - it is required
    /// nothing, and may block another or none. An attacker named is the object it is: one that left combat and came back is a
    /// new object it never names (CR 400.7). An effect, not an ability, as `MustBlock` is.
    MustBlockAttacker(ObjectId),
    /// "Its activated abilities can't be activated" (CR 602.5) for as long as the effect lasts: every activated ability of
    /// the object, mana abilities (CR 605.1a) and abilities whose cost untaps it included, whoever would activate them. A
    /// resolved effect, so it outlasts its source (Braided Net).
    CantActivateAbilities,
    /// "Power-up abilities can't be activated" (CR 602.5, 702.193) while the effect applies: every power-up ability
    /// (`ActivatedDef::power_up`) of every permanent, whoever controls it and whoever would activate it, a permanent that arrives
    /// later included. A rule about the game, not an object: it outlives its `affected` object, the source (CR 611.2c). With
    /// `Duration::EndOfExtraTurn` it applies in that extra turn only.
    CantActivatePowerUp,
    /// "Can't attack" (CR 508.1c) for as long as the effect lasts: a restriction on declaring the creature as an attacker.
    CantAttack,
    /// "It can't be sacrificed" (CR 101.2, 701.21a) for as long as the effect lasts (Slicer, Hired Muscle's "this turn"): as
    /// `Keyword::CantBeSacrificed`, but an effect, not an ability, so a permanent that loses its abilities is still under it.
    CantBeSacrificed,
    /// "It can't become untapped" (CR 101.2, 701.26b) for as long as the effect lasts: it stays tapped in the untap step and
    /// when an effect would untap it, and a stun counter on it stays (CR 122.1d). As `CardDef::cant_become_untapped`, but a
    /// resolved effect of an ability on the permanent it names, so it is no ability of that permanent (CR 613.1f).
    CantBecomeUntapped,
    /// The permanent is harnessed (CR 701.64): a designation it keeps as long as it stays where it is
    /// (`Duration::Indefinite`), which turns its infinity abilities on (CR 702.186).
    Harnessed,
    /// The permanent is suspected (CR 701.60): a designation it keeps until it leaves the battlefield or an effect ends it
    /// (`Duration::Indefinite`), neither an ability nor a copiable value (CR 701.60b). While it lasts the permanent has
    /// menace and "This creature can't block" (CR 701.60c), given in layer 6 at the effect's timestamp, so "loses all
    /// abilities" later takes them though it stays suspected (the rulings).
    Suspected,
    /// The effect's controller has hexproof (CR 702.11c): their opponents' spells and abilities can't target them. It
    /// is about the player and outlives its `affected` object, which is only the source.
    PlayerHexproof,
    /// Each creature the effect's controller controls with toughness greater than its power assigns combat damage equal to its
    /// toughness rather than its power (CR 510.1a, 611.2c) while it lasts: a rule about the player's creatures as damage is
    /// assigned, so it reaches one that arrives later. The Kingpin of Crime's "until end of turn". The same rule as a permanent's
    /// static ability is `CardDef::damage_by_toughness`. Outlives its `affected` object, the source.
    CreaturesAssignDamageByToughness,
    /// The effect's controller has protection from everything (CR 702.16j): nothing can target them, whoever controls it,
    /// and damage from any source is prevented. About the player, not its `affected` object, which is only the source.
    PlayerProtectionFromEverything,
    /// "You may cast [spells of these types] as though they had flash" (CR 307.5, 702.8): the effect's controller may
    /// cast them whenever they could cast an instant. Outlives its `affected` object, the source.
    CastAsThoughFlash(TypeSet),
    /// "You may activate loyalty abilities of [Jace] planeswalkers you control on any player's turn any time you could cast
    /// an instant" (CR 606.3, 611.2c; Jace's Machinations): the effect's controller may activate the loyalty abilities of the
    /// planeswalkers they control that have this planeswalker type (every planeswalker for `None`) whenever they have
    /// priority. Only the timing changes: one loyalty ability per permanent each turn still holds. A rule about the player,
    /// so it reaches planeswalkers that arrive later and outlives its `affected` object, which is only the source.
    /// It is in the core, though one printed card has it as a resolved effect, because it must outlive the spell, belong to
    /// the player and reach a planeswalker that does not exist yet, and only the core checks timing. The other cards that
    /// grant this (Teferi, Master of Time; The Wandering Emperor; the Teferi emblems) are statics: `CardDef.instant_activation`.
    InstantLoyalty(Option<Subtype>),
    /// The effect's controller has this maximum hand size, `None` for none (CR 402.2), for as long as it lasts: Spirit Water
    /// Revival's "no maximum hand size for the rest of the game" (`Duration::Indefinite`). It is ordered by its timestamp with
    /// the permanents' `PlayRule::MaximumHandSize` (CR 613.11). About the player, so it outlives its `affected` object, which
    /// is only the source.
    MaximumHandSize(Option<u32>),
    /// "You may play [N] additional land[s] this turn" (CR 305.2, 611.2c): the effect's controller may play N more lands than
    /// they otherwise could for as long as it lasts (`Duration::EndOfTurn`). Added to every other such effect and to the
    /// static abilities that do the same (`PlayRule::ExtraLandPlays`): the number of lands a player can play is the sum
    /// (CR 305.2), compared with the lands they have already played this turn, however they were played (CR 305.2a/b), so
    /// a land play made before the effect began still leaves the new one. It counts only lands played, never lands put onto
    /// the battlefield (CR 305.4). Explore, Urban Evolution, Scale the Heights, Kiora, the Crashing Wave's −1, Way of the
    /// Paradox; "up to two" (Journey of Discovery) is 2, "X additional lands" (Nahiri's Lithoforming) is X, so the count is
    /// 32-bit. About the player, so it outlives its `affected` object, which is only the source.
    ExtraLandPlays(u32),
    /// The player can't gain life (CR 119.7): Screaming Nemesis's "for the rest of the game" (`Duration::Indefinite`). It is
    /// about the player and outlives its `affected` object, which is only the source.
    CantGainLife(PlayerId),
    /// The player's life total can't change (CR 119.7, 119.8; Teferi's Protection's "until your next turn"): they gain no
    /// life and lose none, a life total set to another number stays as it is, and a cost of more than 0 life can't be paid
    /// (the rulings). Damage is still dealt to them; only the loss of life it would cause doesn't happen. About the player,
    /// so it outlives its `affected` object, which is only the source.
    LifeTotalCantChange(PlayerId),
    /// The player can't cast spells from these zones (CR 101.2, 601.3, 611.2c): Avatar's Wrath's "until your next turn, your
    /// opponents can't cast spells from anywhere other than their hands", one effect for each player affected, who are the
    /// opponents as it resolves. Only casting is stopped: an activated ability is no spell, and a copy of a spell put onto
    /// the stack is not cast (CR 707.10). It is about the player, so it reaches cards that arrive in those zones later, and
    /// outlives its `affected` object, which is only the source. A static ability with the same words (Drannith Magistrate)
    /// is a `CardDef::cast_rules` entry, which is told the zone too.
    CantCastSpellsFrom(PlayerId, ZoneKinds),
    /// The creature can attack as though it didn't have defender (CR 702.3b, 609.4) for as long as the effect lasts.
    CanAttackDespiteDefender,
    /// "Creatures [this player] controls can't block this turn" (CR 509.1b, 611.2c): a rule about a player, so it reaches creatures
    /// that arrive later too and grants no ability. Its `affected` object is only the source.
    CreaturesCantBlock(PlayerId),
    /// A regeneration shield (CR 701.19a, 614.8): the next time the permanent would be destroyed this turn, it is
    /// tapped, its damage removed and it is removed from combat instead. One effect for each shield.
    RegenerationShield,
    /// A prevention effect (CR 615) a resolving spell or ability made, for as long as its duration lasts: the nth of the
    /// `granted_preventions` of `card`, the card whose spell or ability made it (the face that was up, for a double-faced
    /// permanent that transforms later), which says what damage it watches for. `to` is whom that damage is dealt to:
    /// an object (its `affected` object too, so the effect ends with it, CR 400.7), a player, or `Target::None` for damage
    /// to anyone ("prevent all combat damage that would be dealt this turn"), when it is about no object and outlives its
    /// `affected` object, which is only the source. `shield` is how much it has left to prevent (CR 615.7).
    Prevention { card: CardId, index: u8, to: Target, shield: Shield },
    /// A replacement effect (CR 611.2, 614) a resolving spell or ability made about its `affected` object, for as long
    /// as its duration lasts: the nth of the source card's `granted_replacements`. It ends with the object, which is a
    /// new one when it changes zones (CR 400.7): "if that creature would die this turn, exile it instead".
    Replacement(u8),
    /// The same, about a player or the game and not an object ("if a card would be put into your graveyard from
    /// anywhere this turn, exile it instead"): its `affected` object is only the source, and it outlives it.
    ReplacementRule(u8),
    /// "You may play lands and cast spells from your graveyard" (CR 305.1, 601.3) for the effect's controller. About the
    /// player, and outlives its `affected` object, which is only the source.
    PlayFromGraveyard,
    /// A mana ability the effect's controller has of their own until the effect ends (CR 605.1a, 611.2a): an ability of the
    /// source card's `delayed` list. About the player, so it outlives its `affected` object, which is only the source (Channel).
    PlayerManaAbility(AbilityRef),
    /// A copy effect (CR 707.2, layer 1) on the `affected` permanent, for its duration (CR 611.2b): it holds the copiable
    /// values it was applied over, put back when it ends (or handed to the next later copy effect on the permanent, CR
    /// 613.7: `GameState::effect_remove`). The copy the latest one makes is in `Obj::copy`.
    RestoreCopy(Option<Copiable>),
    /// A text-changing effect (CR 612, layer 3): every instance of `from` in the object's rules text and type line is
    /// `to` (Trait Doctoring, Artificial Evolution). Card code reads its words through `Query::words`; the layers change
    /// the type line.
    ChangeText { from: Word, to: Word },
    /// "[It] becomes ... named [name]" (CR 612.8): a text-changing effect (layer 3) that sets the object's name, so it
    /// loses any names it had and has only this one, for every rule and effect that asks its name (the legend rule,
    /// "with the same name", a chosen name). Its rules text is not changed: text that refers to the object by name still
    /// means the object (CR 201.5). The Irencrag's "become a legendary Equipment artifact named Everflame, Heroes'
    /// Legacy".
    SetName(&'static str),
    /// "The next spell you cast this turn can't be countered" (CR 611.2c, 601.2, 701.5a; Mistrise Village): a rule about the
    /// effect's controller that reaches a spell that does not exist yet, so it outlives its `affected` object, which is only the
    /// source. It marks the next spell that player casts, as it is cast and before anyone has priority, and is used up by it.
    /// It is in the core, though one printed card has it, because only the core is there while a spell is being cast: a
    /// card file can't mark a spell during casting, and a delayed trigger would go on the stack above the spell and could
    /// be answered. A kind of spell ("the next creature spell") is a filter to add when a printed card needs one.
    NextSpellCantBeCountered,
    /// "[This player] skips their next turn" (CR 500.11, 614.10): a replacement effect of the beginning of that player's
    /// next turn, used up by the turn it skips. "Skips their next N turns" is N of them, each waiting for the first turn
    /// the ones before it have not skipped (CR 614.10a). About the player, so it outlives its `affected` object, which is
    /// only the source.
    SkipNextTurn(PlayerId),
    /// "[The effect's controller] controls [player] during that player's next turn" (CR 723.1), waiting for it: the next
    /// turn `player` actually takes, so a skipped turn leaves it waiting (723.1b). As that turn begins every such effect on
    /// the player is used up, each becoming the control of them for the turn (`ControlsPlayer`), the last created
    /// whose controller is in the game the one that works (723.1a). `then_extra_turn` is "After that turn, that player
    /// takes an extra turn" (Emrakul, the Promised End), which each such effect gives, an overwritten one too (Emrakul's
    /// ruling), and one whose controller has left the game too. About the player, so it outlives its `affected` object,
    /// which is only the source. Mindslaver, Sorin Markov, Worst Fears.
    ControlNextTurn { player: PlayerId, then_extra_turn: bool },
    /// The effect's controller controls `player` (CR 723): they make every choice and decision `player` is allowed or told
    /// to make (723.5), with `player`'s resources (723.5a), and see what `player` sees of the game (723.4); `player` is still
    /// the active player and controls their objects (723.3). Who is asked is unchanged - the request is `player`'s - and
    /// the host routes it (`GameState::player_controller`). Made by `ControlNextTurn` as that turn begins, it lasts until
    /// the next turn begins (723.1) - through any turn skipped meanwhile, which never begins - and ends at once if its
    /// controller leaves the game (CR 800.4a).
    ControlsPlayer(PlayerId),
    /// "The 'legend rule' doesn't apply to permanents [this player] controls" (CR 704.5j, 611.2c) for as long as it lasts:
    /// Hall of Echoes's "this turn". About the player, so it reaches legends that arrive later and outlives its `affected`
    /// object, which is only the source. When it ends, the legend rule applies at once to what they control (its ruling).
    /// The static abilities with these words (Mirror Box, Brothers Yamazaki's pair) are not built.
    NoLegendRule(PlayerId),
    /// "Artifact spells you cast this turn cost {1} less to cast" (CR 611.2c, 601.2f, 118.7): a cost change a resolved spell
    /// or ability made, which changes the total cost of the spells `player` casts for as long as it lasts, whether or not
    /// its source is still on the battlefield (CR 611.2a). It is the `index`th of the
    /// `lasting_cost_changes` of `card`, the card whose spell or ability made it, which says which spells and by how much.
    /// About the player, so it reaches spells cast later and outlives its `affected` object, which is only the source.
    SpellCostChange { card: CardId, index: u8, player: PlayerId },
    /// It has the enchant ability (CR 702.5a, layer 6) that `card` gives the objects its effects make Auras
    /// (`CardDef::becomes_aura_enchant`): what it may enchant as it enters (CR 303.4f) and for as long as it is attached
    /// (CR 303.4c, 704.5m). Necromancy's "it becomes an Aura with 'enchant creature put onto the battlefield with
    /// Necromancy'", Old-Growth Troll's "It's an Aura enchantment with enchant Forest you control" (`EntersAs`).
    Enchant(CardId),
}

impl EffectKind {
    /// Whether the effect is about a player or a rule, not its `affected` object: it goes on when that object is gone
    /// (CR 611.2c).
    pub fn outlives_affected(self) -> bool {
        matches!(
            self,
            EffectKind::PlayerHexproof
                | EffectKind::CantActivatePowerUp
                | EffectKind::PlayerProtectionFromEverything
                | EffectKind::CastAsThoughFlash(_)
                | EffectKind::InstantLoyalty(_)
                | EffectKind::MaximumHandSize(_)
                | EffectKind::ExtraLandPlays(_)
                | EffectKind::CantGainLife(_)
                | EffectKind::LifeTotalCantChange(_)
                | EffectKind::CantCastSpellsFrom(..)
                | EffectKind::ReplacementRule(_)
                | EffectKind::CreaturesCantBlock(_)
                | EffectKind::CreaturesAssignDamageByToughness
                | EffectKind::PlayFromGraveyard
                | EffectKind::PlayerManaAbility(_)
                | EffectKind::NextSpellCantBeCountered
                | EffectKind::SkipNextTurn(_)
                | EffectKind::ControlNextTurn { .. }
                | EffectKind::ControlsPlayer(_)
                | EffectKind::NoLegendRule(_)
                | EffectKind::SpellCostChange { .. }
        ) || matches!(self, EffectKind::Prevention { to, .. } if to.object().is_none())
    }
}

/// How much damage a prevention effect a spell or ability made prevents (CR 615.1, 615.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shield {
    /// All of the damage it applies to, for as long as it lasts: "prevent all damage that would be dealt to target
    /// creature this turn".
    All,
    /// The next N damage (CR 615.7): each 1 damage it prevents reduces it by 1, and once it is 0 the rest is dealt.
    Next(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Duration {
    /// Until the cleanup step (CR 514.2).
    EndOfTurn,
    /// "This combat" (CR 506.1, 511.3): until the combat phase it began in ends - so an additional combat phase that turn
    /// (CR 500.8, Aurelia, the Warleader) begins without it; one made outside combat ends with the turn's cleanup at the
    /// latest. Ursine Monstrosity's "attacks that player this combat if able".
    EndOfCombat,
    /// As long as the affected object stays where it is.
    Indefinite,
    /// "For as long as [the source] remains on the battlefield" (CR 611.2b): it ends the moment the effect's source is no
    /// longer the permanent it was, on the battlefield, or phases out (CR 702.26f) - as that event happens, in the middle of
    /// a cost being paid or a resolution as much as anywhere, not when the state-based actions are next checked.
    WhileSourceOnBattlefield,
    /// Until the next turn of this player begins (CR 611.2b): "until your next turn".
    UntilNextTurnOf(PlayerId),
    /// "Until the end of your next turn" (CR 611.2a, 514.2): the effect lasts through the next turn of this player to begin, whether
    /// or not it is the turn it began in, and ends in that turn's cleanup step. It becomes `EndOfTurn` as that turn begins; a player
    /// who has left the game would have begun it and the effect ends then (CR 800.4k, 800.4m). Evil's Thrall's "gain control of
    /// that creature until the end of your next turn".
    EndOfNextTurnOf(PlayerId),
    /// "During that turn" (CR 500.7, 611.2a): an effect made before an extra turn that is about that turn only, the one with this
    /// id (`ExtraTurn::id`, `Turn::extra`), and about the whole of it, its cleanup step's priority included. It applies to nothing
    /// before that turn, nor in any other extra turn or the turn it was made in, and ends as the next turn begins - or as it would
    /// have begun, if the turn is never taken because its player has left the game (CR 800.4k-m). Only a reader that asks whether
    /// that turn is the current one (`Turn::extra`) makes the effect wait for it, so only an effect such a reader reads may last
    /// so: `EffectKind::CantActivatePowerUp`, in `activation_barred`. Kang the Conqueror's "during that turn, power-up abilities
    /// can't be activated".
    EndOfExtraTurn(u32),
    /// "During its controller's next turn" (CR 611.2a): the effect does something only during the first turn the affected
    /// object's controller - whoever controls it then - begins after both the turn they gained it and the turn the effect
    /// began in, `began`, which the core
    /// sets as the effect begins (a card passes `Duration::DURING_ITS_CONTROLLERS_NEXT_TURN`). The turn it began in is never
    /// that turn. A change of control before the creature has had its chance to attack makes it the new controller's next
    /// turn instead (Gideon, Battle-Forged's ruling); once its chance has come - attackers declared, or the turn's cleanup
    /// - the effect lasts until end of turn and no further, whoever controls the creature next. Every declare attackers
    /// step of that turn is under it (CR 508.1d). Until then it waits, however long. Only the declaration of attackers
    /// reads whether it is that turn, so only an effect on it may last so (`EffectKind::MustAttackAt`, Gideon,
    /// Battle-Forged; `EffectKind::CantAttack`, Wall of Dust's "can't attack during its controller's next turn").
    DuringControllersNextTurn(u32),
    /// "Until this card is cast from exile" (CR 611.2a, 400.7): until this object, a card in exile, becomes cast (CR
    /// 601.2i) - after its costs are paid, so a mana ability the effect grants may help pay for it. A card that leaves
    /// exile any other way is a new object (CR 400.7) that can never be cast as this one, so the effect then lasts as long
    /// as its affected object does. Emrakul, the Exigent Doom and Streets of New Capenna's Glamorous Outlaw cycle.
    UntilCastFromExile(ObjectId),
    /// "For as long as it has a [kind] counter on it" (CR 611.2b): it ends the moment the affected object has no counter
    /// of that kind, and does not begin if it has none then; a counter put on later does not bring it back. Eluge, the
    /// Shoreless Sea, Aquitect's Will, Quicksilver Fountain and Xolatoyac, the Smiling Flood ("it's an Island ... for as
    /// long as it has a flood counter on it"), Aven Mimeomancer, Mathas, Fiend Seeker.
    WhileCounter(CounterKind),
    /// "For as long as it remains tapped" (CR 611.2b): it ends the moment the affected permanent untaps, and does not
    /// begin if it is untapped then. Braided Net; Rust Tick, Amber Prison and the other "doesn't untap ... for as long as"
    /// cards word theirs about their own source instead.
    WhileTapped,
    /// "For as long as [condition]" (CR 611.2b), the condition the card's own: the `index`th of its `lasting` list, asked of
    /// the effect whenever the state-based actions are checked. It ends the first time the condition is false, and does
    /// nothing if it is false as the effect would begin; it never comes back. Old Man of the Sea's "for as long as this
    /// creature remains tapped and that creature's power remains less than or equal to this creature's power".
    AsLongAs(CardId, u8),
}

impl Duration {
    /// "During its controller's next turn", the turn it began in set as the effect begins (`DuringControllersNextTurn`).
    pub const DURING_ITS_CONTROLLERS_NEXT_TURN: Duration = Duration::DuringControllersNextTurn(0);

    /// Whether an effect of this duration on `affected` still lasts as far as the object itself goes (CR 611.2b): false
    /// once the condition a "for as long as" duration names about it is no longer true.
    pub fn holds_for(self, affected: &Obj) -> bool {
        match self {
            Duration::WhileCounter(kind) => affected.counters(kind) > 0,
            Duration::WhileTapped => affected.tapped,
            _ => true,
        }
    }
}

/// When a permission to play a card ends (CR 611.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Expiry {
    /// For as long as the card stays where it is: a card that changes zones is a new object (CR 400.7).
    Never,
    /// "This turn": the cleanup step (CR 514.2).
    EndOfTurn,
    /// "Until the end of your next turn": the cleanup step of the permitted player's next turn.
    EndOfYourNextTurn,
    /// "Until your next end step" (CR 500.4): ends as the holder's next end step begins, before its triggers.
    UntilYourNextEndStep,
    /// "For as long as you control [this permanent]" (CR 611.2b): it lasts while the holder controls this object on the
    /// battlefield, and ends for good the first time they don't - the permanent leaves (a returned one is a new object,
    /// CR 400.7), phases out (CR 702.26f) or changes controller, though it come back - and it does nothing if that is
    /// already so as it would begin. Transforming it ends nothing: it is the same object (CR 712.18). Gwen Stacy and
    /// Lightning, Security Sergeant (exile the top card of your library, play it) print it; Victor Mancha, Runaway, Taster
    /// of Wares, Hama, the Bloodbender and Kotose, the Silent Spider word theirs about other cards they exile.
    WhileControls(ObjectId),
}

impl Expiry {
    /// The permanent a "for as long as you control" permission is about (`Expiry::WhileControls`), if it is one.
    pub fn controlled(self) -> Option<ObjectId> {
        match self {
            Expiry::WhileControls(source) => Some(source),
            _ => None,
        }
    }
}

/// What a permission to play a card is, apart from who holds it and for which card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayGrant {
    /// The cost paid instead of the card's mana cost (CR 118.9): `Some(ManaCost::ZERO)` casts it for free.
    pub instead: Option<ManaCost>,
    /// "Play", not "cast" (CR 305.1): a land among them may be played as a land play too.
    pub lands: bool,
    pub expiry: Expiry,
    /// Mana of any type can be spent to cast it (CR 609.4b, 118.14): any mana pays any symbol of its cost, which is not
    /// changed (`ManaCost::paid_with_any_type`).
    pub any_type: bool,
    /// Generic mana added to what casting it costs (CR 601.2f): Elite Spellbinder's "costs {2} more to cast".
    pub more: u8,
    /// It may be played only on a turn after the one it was granted on: warp's "cast this card from exile on a later
    /// turn" (CR 702.185a).
    pub later_turns: bool,
    /// It is flashback (CR 702.34a): an alternative cost, the card's mana cost unless `instead` names another, and a spell
    /// cast this way is exiled instead of put anywhere else any time it would leave the stack. A flashback granted by
    /// Snapcaster Mage or Flashback.
    pub flashback: bool,
    /// "If that spell would be put into your graveyard, exile it instead" (CR 614.1a): no alternative cost, and only a move
    /// to the graveyard is replaced (`ExileInstead`). Jace, Telepath Unbound's "you may cast target instant or sorcery card
    /// from your graveyard this turn".
    pub exile_instead: ExileInstead,
    /// It may be played only while this holds (CR 611.2, 601.3): "during any turn you attacked with a Rogue".
    pub when: Option<PlayCondition>,
    /// It is cast only at sorcery speed, whatever it is (CR 702.170d): plot's "cast it as a sorcery".
    pub sorcery_speed: bool,
    /// "Each land played this way enters tapped" (CR 305.1, 614.1c): a replacement effect on the land's entering, made by
    /// the effect that gave the permission and ordered with the others that would change the same entry (CR 616.1).
    /// Lightstall Inquisitor.
    pub enters_tapped: bool,
    /// It may be cast only as its Adventure (CR 715.3, 715.3d): Mosswood Dreadknight's "you may cast it from your graveyard as
    /// an Adventure". Its card as itself, or another face, is not cast by this permission.
    pub adventure_only: bool,
    /// A spell cast by it is paid for with life equal to its mana value rather than its mana cost (CR 118.9, 119.4): Inside
    /// Information's "pay life equal to its mana value rather than pay its mana cost", as Bolas's Citadel's rule pays for the
    /// top of a library. An X in its cost is 0 (CR 107.3), and no other alternative cost may be used with it (CR 118.9a).
    pub life_for_mana_value: bool,
}

/// "If [that spell] would be put into your graveyard, exile it instead" (CR 614.1a): the replacement effect an effect that
/// lets a spell be cast, or casts it, makes of that spell. It replaces only the spell's move from the stack to its owner's
/// graveyard - resolving, countered, or with its targets gone - and not a move to a hand or a library, which flashback's
/// "instead of putting it anywhere else" replaces too (CR 702.34a, `StackItem::flashback`). It is ordered with the other
/// replacement effects that would change the same move by the spell's controller (CR 616.1). "Your graveyard" is the
/// owner's: each card that says it has the spell cast from its controller's own graveyard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ExileInstead {
    /// No such effect.
    #[default]
    Never,
    /// "If that spell would be put into your graveyard, exile it instead", whatever the spell is: Dreadhorde Arcanist, Jace,
    /// Telepath Unbound.
    Spell,
    /// "If an instant or sorcery spell cast this way would be put into your graveyard, exile it instead": only while the
    /// spell is an instant or a sorcery, judged as it would be put there (Bilbo, Thief in the Night, whose cast may be an
    /// artifact spell).
    InstantOrSorcery,
}

/// A condition on a standing permission to play a card, worked out whenever the permission is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayCondition {
    /// A creature of this type was declared as an attacker by the permitted player this turn (CR 508.1): Robber of the Rich.
    AttackedWithThisTurn(Subtype),
    /// The permitted player controls a permanent of this subtype now (CR 205.3, 110.2): Flameshape's "you may play them if you
    /// control a Wizard". Asked as the card would be played (CR 601.3, 305.1); losing the permanent once a spell is cast does
    /// not undo it (ruling 2026-06-29).
    ControlsSubtype(Subtype),
}

impl PlayGrant {
    /// "You may cast it": for as long as it stays where it is, for its mana cost.
    pub const CAST: PlayGrant = PlayGrant { instead: None, lands: false, expiry: Expiry::Never, any_type: false, more: 0, later_turns: false, flashback: false, exile_instead: ExileInstead::Never, when: None, sorcery_speed: false, enters_tapped: false, adventure_only: false, life_for_mana_value: false };
}

/// A standing permission to play or cast one card from a zone it could not be played from (CR 601.3, 305.1): "until
/// the end of your next turn, you may play that card", airbend's "may cast it for {2}".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayPermission {
    pub card: ObjectId,
    pub player: PlayerId,
    pub grant: PlayGrant,
    /// The spell or ability whose effect gave it (CR 601.3): what a cast or land play by it is recorded as made by, and
    /// how an action names it among several permissions for the same card.
    pub by: Grant,
    /// The turn it was granted on, for `Expiry::EndOfYourNextTurn`.
    pub turn: u32,
}

/// A triggered ability that has triggered and waits to be put on the stack (CR 603.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PendingTrigger {
    pub source: ObjectId,
    pub ability: AbilityRef,
    pub controller: PlayerId,
    /// What the event was about (the player or object "it" and "that player" refer to): no target, never checked.
    pub subject: Target,
    pub data: [u32; 2],
    /// The batch it was triggered by, for a "one or more" ability (CR 603.2c).
    pub members: Span,
    /// For a reflexive triggered ability whose effect counts what its first part did ("tap up to that many target
    /// creatures", CR 603.12): that number. A `TargetSpec::COUNTED` bound reads it, and it is the stack item's `x`.
    pub count: Option<u16>,
    /// For a delayed triggered ability, the history's head when it was created (`Delayed::since`), which its stack item
    /// keeps (CR 701.27f); none for any other, whose item takes the head as it is put on the stack.
    pub since: Option<u32>,
    /// How its source acquired the ability (CR 607.5): its stack item's `acquired`.
    pub acquired: Acquired,
}

/// A run of `GameState::members`: the objects of one batch of simultaneous events, kept for the ability they
/// triggered (CR 603.2c, 608.2h). Its length is a `u32`, as a batch can hold every object of the game (RX-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub len: u32,
}

/// A delayed triggered ability waiting for its moment (CR 603.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Delayed {
    pub when: DelayedWhen,
    pub source: ObjectId,
    pub controller: PlayerId,
    /// The ability that triggers: a card's delayed ability (`AbilityRef::delayed`).
    pub ability: AbilityRef,
    pub subject: Target,
    /// The objects the ability is about (CR 603.7c), kept in `GameState::members`: a mobilize ability's Warriors, which it
    /// sacrifices together. `StackItem::members` once it triggers.
    pub members: Span,
    /// The history's head when it was created: "since that delayed triggered ability was created" (CR 701.27f).
    pub since: u32,
    /// How the source of the ability that created it acquired that ability (CR 607.5): what it is linked by as it resolves.
    pub acquired: Acquired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DelayedWhen {
    /// At the beginning of the next such step, of anyone's turn or one player's. Once.
    Step { step: Step, on_turn_of: Option<PlayerId> },
    /// "At the beginning of each [step] ... for the rest of the game" (CR 603.7, 702.192a): at every such step, of anyone's
    /// turn or one player's, and never used up. It is no ability of an object, so it goes on whatever happens to the object
    /// it came from (paradigm's ruling: "it doesn't matter what happens to the card in exile").
    EachStep { step: Step, on_turn_of: Option<PlayerId> },
    /// "Whenever [event] this turn" (CR 603.7b): triggers each time the ability's own `listens`/`matches` accept an
    /// event, until the cleanup step ends the turn. The ability's `matches` is asked of `source`.
    ThisTurn,
    /// "Until [event]" (CR 610.3, 603.7): triggers once, the next time its `listens`/`matches` accept an event, whenever that is, and is
    /// then used up. The ability's `matches` is asked of `source`, which may be gone by then.
    UntilEvent,
    /// "Until your next turn, whenever [event]" (CR 603.7b, 611.2b): as `ThisTurn`, until that player's next turn begins.
    UntilNextTurnOf(PlayerId),
    /// "When you next [event] this turn" (CR 603.7b): triggers once, the first time its `listens`/`matches` accept an event
    /// this turn, and is then used up; one that never triggers ends with the turn, as `ThisTurn` does. Each is its own
    /// ability: two made the same turn both trigger on the next such event (Doublecast's ruling). Doublecast, Teach by
    /// Example, Galvanic Iteration, Chandra, the Firebrand, Ral, Storm Conduit and Loki Laufeyson print it.
    NextThisTurn,
    /// "At the beginning of that turn's [step]" for an extra turn an effect created (CR 500.7, 603.7): Final Fortune, Last
    /// Chance, Warrior's Oath. Triggers once, at that step of that turn alone (`Turn::extra`); if that turn ends without
    /// the step (Sundial of the Infinite) or is never taken (skipped, CR 500.11), it never triggers, and it is dropped as
    /// the next turn begins.
    ExtraTurnStep { step: Step, turn: u32 },
}

impl DelayedWhen {
    /// Whether it triggers on events, as many times as they happen, rather than once at a step.
    pub fn is_event(self) -> bool {
        matches!(self, DelayedWhen::ThisTurn | DelayedWhen::UntilEvent | DelayedWhen::UntilNextTurnOf(_) | DelayedWhen::NextThisTurn)
    }
}

/// The most abilities an object has at once, printed and granted (`Chars::abilities`, `LastKnown::abilities`): enough for a
/// Locus of Enlightenment or an Agatha's Soul Cauldron's creature to have every activated ability of the cards it takes them
/// from (CR 702.167c), and the width of `Obj::activated_mask`.
pub const MAX_ABILITIES: usize = 32;

/// An object's characteristics as it last existed (CR 608.2h, 603.10, 400.7), kept when it ceases to be that object, or
/// as a spell became cast (`StackItem::cast_known`, CR 601.2i): what the layers compute, which its row cannot give
/// afterwards. The controller is captured too: a same-identity phased-out object's live control may later change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LastKnown {
    pub id: ObjectId,
    pub card: CardId,
    pub controller: PlayerId,
    /// Its name as it last had it (CR 201.2): a copy's, a face-down permanent's none, or one an effect set (CR 612.8).
    pub name: &'static str,
    /// An effect set its name (`Chars::name_set`).
    pub name_set: bool,
    pub supertypes: Supertypes,
    /// It had no mana cost (`Chars::no_cost`).
    pub no_cost: bool,
    pub types: TypeSet,
    pub subtypes: SubtypeSet,
    pub colors: ColorSet,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub keywords: Keywords,
    /// Its total toxic value as it last had it (CR 702.164b).
    pub toxic: u16,
    pub protection: ColorSet,
    pub protection_colorless: bool,
    pub protection_players: u8,
    pub protection_types: TypeSet,
    /// Its protection from creature types as it last had it (`Chars::protection_subtypes`).
    pub protection_subtypes: SubtypeSet,
    /// Its protection from a mana value at least this, as it last had it (`Chars::protection_mana_value`).
    pub protection_mana_value: Option<u32>,
    pub spell: bool,
    pub protection_colored_spells: bool,
    pub protection_everything: bool,
    pub hexproof_from: HexproofFrom,
    pub must_attack: u32,
    /// It was an attacking creature (CR 506.3) as it last was on the battlefield: combat forgets a permanent that leaves it, so
    /// "whenever an attacking creature you control dies" asks this (CR 603.10a).
    pub attacking: bool,
    /// Its mana value as it last had it (CR 202.3; on the stack, X counted, CR 202.3e): a cast's is its mana value as it
    /// became cast (`history::Cast::value`).
    pub mana_value: u32,
    /// Its abilities as it last had them, those granted by effects included: a leaves-the-battlefield ability it was given still
    /// triggers (CR 603.10a).
    pub abilities: [Option<AbilityRef>; MAX_ABILITIES],
    /// The words of its text as it last had them (CR 612): an ability of it that resolves after it has gone reads them.
    pub words: Words,
}

/// A player (CR 102).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Player {
    pub life: i32,
    /// The life total the player began the game with (CR 103.4): "half your starting life total".
    pub starting_life: i32,
    pub pool: ManaPool,
    /// How much of each kind in the pool lasts until the end of combat (CR 106.4, 500.5a): firebending's mana. Spent
    /// last, since the ordinary mana of a kind is spent first, which is never worse for the player; a count above
    /// what the pool holds is stale, and `lasting_mana` clamps it.
    pub lasting: ManaPool,
    /// Lands played this turn (CR 305.2): the rules' own count, which "may play an additional land" is checked against at
    /// every priority and the loop watch fingerprints with the player, so it is kept here rather than asked of the history.
    pub lands_played: u32,
    /// Drew from an empty library since the last state-based action check (CR 704.5b).
    pub drew_from_empty: bool,
    pub lost: bool,
    pub mulligans: u8,
    pub kept: bool,
    /// The number of the turn this player's most recent turn was.
    pub turn_began: u32,
    /// Poison counters (CR 122.1f): ten lose the game (CR 704.5c). Private, as an object's counters are (`Obj::counters`):
    /// read through `poison`, put on only by the put path.
    poison: u8,
    /// Energy counters (CR 107.14, 122.1): a player's own store, gained and paid (`pay_energy`).
    energy: u8,
    /// Experience counters (CR 122.1): a player's own store, only ever gained.
    experience: u8,
    /// The city's blessing (CR 702.131): had for the rest of the game once gained.
    pub citys_blessing: bool,
    /// An enduring story (CR 702.195b): had for the rest of the game once gained.
    pub enduring_story: bool,
    /// The monarch (CR 725): at most one player is, once a game has had one.
    pub monarch: bool,
    /// The player with the initiative (CR 726): at most one.
    pub initiative: bool,
    /// The room of the Undercity this player's venture marker is on (CR 309.4, 701.49): 0 for none, else the room's
    /// index plus one.
    pub room: u8,
    /// The player's speed (CR 702.179): none until a rule or effect sets it (CR 702.179b), 1 once they control a
    /// permanent with start your engines! (CR 704.5aa), and up from there to 4. Nothing decreases it.
    pub speed: Option<u8>,
    /// The card with companion this player revealed from outside the game before the game began (CR 103.2b): their chosen
    /// companion, public while it stays there (CR 702.139a). Once it has been put into their hand - by the special action
    /// (CR 116.2g) or by anything else that brings it in - it is a new object (CR 400.7) and this names what it was, so the
    /// action, offered only while this card is still outside the game, is taken at most once.
    pub companion: Option<ObjectId>,
    /// The companion chosen as the deck was built whose condition the starting deck fulfils, dealt into the sideboard and
    /// revealed once the starting player is known (CR 103.2b), when it becomes `companion`. None from then on.
    pub companion_to_reveal: Option<ObjectId>,
}

/// Mana in a player's pool with a restriction on what it may be spent on (CR 106.6): one entry per player and
/// restriction, kept in `GameState::restricted_mana` apart from the player, so that a player stays plain data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RestrictedMana {
    pub player: PlayerId,
    pub restriction: ManaRestriction,
    pub pool: ManaPool,
}

/// Where some of the mana in a player's pool came from (CR 106.3): the source that produced it, of one restriction or
/// none (CR 106.6) and lasting until end of combat or not (CR 106.4). The breakdown of the pools by source, kept in
/// `GameState::mana_from` beside the pools themselves (`Player::pool`, `GameState::restricted_mana`), which the entries of a
/// player add up to: written only by the state's mana methods, which write both. An entry whose mana is all spent is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ManaFrom {
    pub player: PlayerId,
    pub restriction: Option<ManaRestriction>,
    /// The object that produced it (CR 106.3), `ObjectId::NONE` for a player's own mana ability (CR 605.1a).
    pub source: ObjectId,
    /// Mana that lasts until end of combat (CR 106.4, 500.5a).
    pub lasting: bool,
    pub pool: ManaPool,
}

/// Which mana of a kind a spending takes, by its source (CR 106.3).
#[derive(Clone, Copy, Debug)]
pub enum ManaPick<'a> {
    /// Any: the mana that does not last first (`Player::spend_mana`'s order), then in the order it came.
    Any,
    /// Mana this source produced.
    From(ObjectId),
    /// Mana none of these sources produced.
    Except(&'a [ObjectId]),
}

impl ManaPick<'_> {
    pub fn takes(&self, source: ObjectId) -> bool {
        match self {
            ManaPick::Any => true,
            ManaPick::From(s) => *s == source,
            ManaPick::Except(not) => !not.contains(&source),
        }
    }
}

/// A limit on what some mana may be spent on (CR 106.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ManaRestriction {
    /// "This mana can't be spent to cast nonartifact spells": it pays artifact spells and everything that is not a
    /// spell.
    NotNonartifactSpells,
    /// "If that mana is spent on a creature spell, it gains haste until end of turn" (CR 106.6; Arena of Glory): it may
    /// be spent on anything, and does that to a creature spell it pays for.
    GainsHaste,
    /// "Spend this mana only to cast a legendary spell, and that spell can't be countered" (CR 106.6, 701.5; Delighted
    /// Halfling): a limit and an effect, each independent of the other, carried by the mana itself.
    LegendaryUncounterable,
    /// "Spend this mana only to cast [type] spells" (CR 106.6): only on casting a spell (CR 601.2g) that has one of these
    /// card types - not on an ability, nor on any other spell, nor on a cost paid for a spell that is not casting it
    /// ("unless its controller pays", CR 118.12). A spell with flashback or kicker is still the spell it is.
    /// Artifact spells: Mishra's Workshop, Steelswarm Operator. Instant or sorcery spells: Great Hall of the Biblioplex,
    /// Tablet of Discovery (and the same limit on Abstract Paintmage and Cormela, Glamour Thief). Planeswalker spells:
    /// Gideon's Memorial. Creature spells: Rockface Village ("a creature spell"), Guildmages' Forum's kind.
    Spells(TypeSet),
    /// "Spend this mana only to activate abilities [of [type] sources | of [type]s]" (CR 106.6, 602.2b): only on the cost
    /// of activating an activated ability - a mana ability's included (CR 605.3a) - whose source has one of these card
    /// types as the ability is activated; with no types, of any source. `AbilitiesOf` says where that source may be: in
    /// any zone for "sources" (a land card's channel or cycling in the hand, CR 109.2c), only on the battlefield for
    /// "[type]s" (CR 109.2). Never on a spell, a special action (CR 116.2: turning a face-down permanent face up,
    /// unlocking a door), a triggered ability's cost or "unless ... pays" (CR 118.12).
    /// Any source: Omen Hawker, Pit Automaton, The Enigma Jewel. Land sources: Sunken Citadel, Eumidian Lifeseed.
    /// Artifact sources: Steelswarm Operator; artifacts: Soldevi Machinist. Creature sources: Shang-Chi, Master of Kung Fu.
    Abilities(TypeSet, AbilitiesOf),
    /// "Spend this mana only to cast [type] spells or activate abilities of [type]s | of [type] sources" (CR 106.6, 601.2g,
    /// 602.2b): casting a spell with one of these card types as it is on the stack, or activating an ability (a mana
    /// ability's included, CR 605.3a) of a source with one of them as it is activated - on the battlefield for "[type]s"
    /// (CR 109.2: not a creature card's cycling in the hand, Castle Garenbrig's ruling), in any zone for "[type] sources"
    /// (CR 109.2c). Never a special action, a triggered ability's cost or "unless ... pays" (CR 118.12).
    /// Creature spells or abilities of creatures: Castle Garenbrig, Lukka, Bound to Ruin; of creature sources: Gwenna, Eyes
    /// of Gaea. Artifact spells or abilities of artifacts: Dalakos, Crafter of Wonders, Grand Architect, Hargilde, Kindly
    /// Runechanter, Power Depot, Renowned Weaponsmith, Slobad, Iron Goblin, Vedalken Engineer.
    SpellsOrAbilities(TypeSet, AbilitiesOf),
    /// "Spend this mana only to cast [a] [subtype] [or [subtype]] [creature] spell[s][, and that spell can't be
    /// countered]" (CR 106.6, 205.3): only on casting a spell that has one of these subtypes as it is on the stack (its
    /// characteristics there, CR 601.2g: a changeling creature spell is every creature type, CR 702.73a; a type an effect
    /// gave it or took away counts); where it says so, also on activating an ability of a source with one of them. See
    /// `SubtypeSpells`.
    Subtypes(SubtypeSpells),
    /// "Spend this mana only to cast [a] non[type] spell[s]" (CR 106.6, 205.2a): only on casting a spell (CR 601.2g) that
    /// has none of these card types as it is on the stack - an artifact creature spell is a creature spell, so no
    /// noncreature spell. Never on an ability. Noncreature: Chandra, Chill of Compliance, Narset of the Ancient Way, The
    /// Emperor of Palamecia, Immortus, Master of Eternity, Nardole, Resourceful Cyborg.
    NonSpells(TypeSet),
    /// "Spend this mana only to cast spells with mana value N or greater [or spells with {X} in their mana costs]"
    /// (CR 106.6, 202.3): only on casting a spell whose mana value on the stack is at least N - with each {X} the number
    /// chosen for it (CR 202.3e, 107.3), a face-down spell's 0 (CR 708.4), the cast half's alone (CR 709.3b) - or, where it
    /// says so, whose mana cost has {X} in it. See `ManaValueSpells`.
    ManaValue(ManaValueSpells),
    /// "Spend this mana only to cast spells from [a zone]" (CR 106.6, 601.2a): only on casting a spell that was cast from
    /// a zone of this kind - the zone the card (or a card's copy, CR 707.12, 722.3) was in as it was put on the stack.
    /// Never on an ability, nor on a spell cast from anywhere else. From exile: Interdimensional Web Watch (any spell
    /// cast from exile, its ruling: not only the cards its own ability exiled).
    SpellsFrom(ZoneKind),
    /// "This mana can't be spent to cast spells from your [zone]" (CR 106.6, 601.2a): everything but casting a spell from
    /// a zone of this kind - an ability, or a spell cast from anywhere else. Only the kind of zone is read: "your hand" is
    /// the hand, as no card in the engine lets a player cast from another player's hand (Sen Triplets would, and would need
    /// the hand's owner compared with the payer). From your hand: Heartwood Crafter (its prepare spell's copy is cast from
    /// exile, CR 722.3).
    NotSpellsFrom(ZoneKind),
}

/// Where the source of an ability that restricted mana may pay for must be (CR 106.6, 109.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbilitiesOf {
    /// "Abilities of [type] sources", or "abilities" alone: a source in any zone (CR 109.2c) - a permanent, or a card in a
    /// hand, a graveyard or exile whose ability functions there.
    Sources,
    /// "Abilities of [type]s": a permanent of that type on the battlefield (CR 109.2) - never a card in another zone.
    Permanents,
}

impl AbilitiesOf {
    /// Whether an ability whose source is or is not on the battlefield is one this reads.
    pub const fn admits(self, on_battlefield: bool) -> bool {
        match self {
            AbilitiesOf::Sources => true,
            AbilitiesOf::Permanents => on_battlefield,
        }
    }
}

/// "Spend this mana only to cast spells with mana value N or greater [or spells with {X} in their mana costs]"
/// (CR 106.6, 202.3, 107.3). Mana value 4 or greater: Ashling, Rimebound. Mana value 5 or greater or {X}: Troyan, Gutsy
/// Explorer. (Helga, Skittish Seer's is for creature spells only, which this does not say.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ManaValueSpells {
    /// The least mana value the spell may have.
    pub at_least: u8,
    /// "... or spells with {X} in their mana costs": a spell whose mana cost has an {X}, whatever its mana value.
    pub or_x: bool,
}

impl ManaValueSpells {
    /// Whether a spell of this mana value, with or without {X} in its mana cost, may be paid for with the mana.
    fn admits(self, mana_value: u32, x: bool) -> bool {
        mana_value >= self.at_least as u32 || (self.or_x && x)
    }
}

/// "Spend this mana only to cast [a] [subtype] [or [subtype]] [creature] spell[s]" (CR 106.6, 205.3).
/// Dragon spells: Desolation of Smaug, Sarkhan, Fireblood. A Dragon spell or an Omen spell: Maelstrom of the Spirit
/// Dragon. A creature spell of the chosen type: Unclaimed Territory, Pillar of Origins, and Cavern of Souls, whose spell
/// can't be countered. Elemental spells or abilities of Elemental sources: Flamebraider. A spell or an ability of a source of
/// the chosen type: Eclipsed Realms; a creature spell or an ability of a creature source of the chosen type: Secluded
/// Courtyard. Elemental spells or abilities of Elementals (permanents): Smokebraider; of Myr: Myr Reservoir; of Dragons:
/// Orb of Dragonkind, Crucible of the Spirit Dragon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SubtypeSpells {
    /// The subtypes, any one of which the spell must have. Read through the words of the text that names them (CR 612)
    /// as the mana is added.
    pub subtypes: [Option<Subtype>; 2],
    /// "Of the chosen type" (CR 205.3m, 607.2a): as the mana is added, `subtypes` becomes the type chosen for the source
    /// as it entered, or nothing at all if none was, which then no spell has.
    pub chosen: bool,
    /// Only a creature spell (CR 302): "a creature spell of the chosen type", "Dragon creature spells"; with `abilities`, only
    /// an ability of a creature source too ("a creature source of the chosen type").
    pub creature: bool,
    /// "... and that spell can't be countered" (CR 106.6, 701.6): the effect the mana has on the spell it pays for.
    pub uncounterable: bool,
    /// "... or activate abilities of [subtype] sources | of [subtype]s" (CR 106.6, 602.2b): it pays too for activating an
    /// ability whose source has one of the subtypes as the ability is activated - of a source in any zone, a permanent or a
    /// card in a hand or graveyard (`AbilitiesOf::Sources`: the rulings of Flamebraider and Eclipsed Realms), or only of a
    /// permanent (`AbilitiesOf::Permanents`, CR 109.2: the rulings of Smokebraider and Myr Reservoir) - never a special
    /// action or a triggered ability's cost. None: spells only.
    pub abilities: Option<AbilitiesOf>,
}

impl SubtypeSpells {
    /// "Spend this mana only to cast a creature spell of the chosen type."
    pub const CHOSEN_TYPE_CREATURE: SubtypeSpells =
        SubtypeSpells { subtypes: [None, None], chosen: true, creature: true, uncounterable: false, abilities: None };

    /// "Spend this mana only to cast a spell of the chosen type or activate an ability of a source of the chosen type"
    /// (Eclipsed Realms).
    pub const CHOSEN_TYPE_SPELLS_OR_ABILITIES: SubtypeSpells =
        SubtypeSpells { subtypes: [None, None], chosen: true, creature: false, uncounterable: false, abilities: Some(AbilitiesOf::Sources) };

    /// "Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the
    /// chosen type" (Secluded Courtyard): `creature` limits the ability's source as it does the spell.
    pub const CHOSEN_TYPE_CREATURE_SPELLS_OR_ABILITIES: SubtypeSpells =
        SubtypeSpells { subtypes: [None, None], chosen: true, creature: true, uncounterable: false, abilities: Some(AbilitiesOf::Sources) };

    /// "Spend this mana only to cast [subtype] spells."
    pub const fn of(subtype: Subtype) -> SubtypeSpells {
        SubtypeSpells { subtypes: [Some(subtype), None], chosen: false, creature: false, uncounterable: false, abilities: None }
    }

    /// "Spend this mana only to cast a [subtype] spell or a [subtype] spell."
    pub const fn either(a: Subtype, b: Subtype) -> SubtypeSpells {
        SubtypeSpells { subtypes: [Some(a), Some(b)], chosen: false, creature: false, uncounterable: false, abilities: None }
    }

    /// "... and that spell can't be countered."
    pub const fn uncounterable(self) -> SubtypeSpells {
        SubtypeSpells { uncounterable: true, ..self }
    }

    /// "... or activate abilities of [subtype] sources": Flamebraider's Elementals.
    pub const fn or_abilities(self) -> SubtypeSpells {
        SubtypeSpells { abilities: Some(AbilitiesOf::Sources), ..self }
    }

    /// "... or activate abilities of [subtype]s", permanents only (CR 109.2): Smokebraider's Elementals, Myr Reservoir's Myr.
    pub const fn or_abilities_of_permanents(self) -> SubtypeSpells {
        SubtypeSpells { abilities: Some(AbilitiesOf::Permanents), ..self }
    }

    /// Whether a spell with these characteristics may be paid for with the mana.
    fn admits(self, types: TypeSet, subtypes: &SubtypeSet) -> bool {
        (!self.creature || types.has(CardType::Creature)) && self.subtypes.iter().flatten().any(|&s| subtypes.has(s))
    }
}

/// What some mana is being spent on, as far as a restriction on it is concerned (CR 106.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManaPayment {
    /// Casting a spell with these card types (CR 601.2g-h). Its subtypes are worked out only where some mana at hand is
    /// restricted by subtype (`ManaRestriction::Subtypes`), and are empty otherwise. `mana_value` is its mana value on the
    /// stack (CR 202.3), and `x` whether its mana cost has {X} in it (CR 107.3); for the advice whether a card could be
    /// cast, its mana value with X at 0, which `x` says could be raised.
    /// `from` is the kind of zone it is cast from (CR 601.2a): where the card is, for the advice; where it was, once on
    /// the stack (`Obj::cast_from`).
    Spell { types: TypeSet, subtypes: SubtypeSet, legendary: bool, mana_value: u32, x: bool, from: ZoneKind },
    /// Activating an ability whose source has these card types and these subtypes as it is activated (CR 602.2b, 605.3a):
    /// both always filled, from the source's characteristics. `on_battlefield`: whether the source is a permanent (CR
    /// 109.2), not a card in a hand, a graveyard or exile.
    Ability { source: TypeSet, subtypes: SubtypeSet, on_battlefield: bool },
    /// Anything else: a special action, "unless ... pays", a cost a resolving effect asks for.
    Other,
}

/// What mana spent on a spell does to it (CR 106.6), as bits of `ManaRestriction::effects`: it gains haste, it can't be
/// countered.
pub const MANA_GIVES_HASTE: u8 = 1;
pub const MANA_MAKES_UNCOUNTERABLE: u8 = 2;

impl ManaRestriction {
    pub const ARTIFACT_SPELLS: ManaRestriction = ManaRestriction::Spells(TypeSet::of(&[CardType::Artifact]));
    pub const INSTANT_OR_SORCERY_SPELLS: ManaRestriction =
        ManaRestriction::Spells(TypeSet::of(&[CardType::Instant, CardType::Sorcery]));
    pub const PLANESWALKER_SPELLS: ManaRestriction = ManaRestriction::Spells(TypeSet::of(&[CardType::Planeswalker]));
    pub const ABILITIES: ManaRestriction = ManaRestriction::Abilities(TypeSet::NONE, AbilitiesOf::Sources);
    pub const LAND_ABILITIES: ManaRestriction = ManaRestriction::Abilities(TypeSet::of(&[CardType::Land]), AbilitiesOf::Sources);
    pub const ARTIFACT_ABILITIES: ManaRestriction = ManaRestriction::Abilities(TypeSet::of(&[CardType::Artifact]), AbilitiesOf::Sources);
    /// "Spend this mana only to activate abilities of artifacts" (Soldevi Machinist): artifact permanents only (CR 109.2).
    pub const ABILITIES_OF_ARTIFACTS: ManaRestriction =
        ManaRestriction::Abilities(TypeSet::of(&[CardType::Artifact]), AbilitiesOf::Permanents);
    pub const NONCREATURE_SPELLS: ManaRestriction = ManaRestriction::NonSpells(TypeSet::of(&[CardType::Creature]));
    pub const CREATURE_SPELLS: ManaRestriction = ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]));
    pub const CREATURE_ABILITIES: ManaRestriction = ManaRestriction::Abilities(TypeSet::of(&[CardType::Creature]), AbilitiesOf::Sources);
    /// "Spend this mana only to cast creature spells or activate abilities of creatures" (Castle Garenbrig, Lukka, Bound to
    /// Ruin): creature permanents' abilities only (CR 109.2).
    pub const CREATURE_SPELLS_OR_ABILITIES_OF_CREATURES: ManaRestriction =
        ManaRestriction::SpellsOrAbilities(TypeSet::of(&[CardType::Creature]), AbilitiesOf::Permanents);
    /// "Spend this mana only to cast creature spells or activate abilities of creature sources" (Gwenna, Eyes of Gaea).
    pub const CREATURE_SPELLS_OR_ABILITIES: ManaRestriction =
        ManaRestriction::SpellsOrAbilities(TypeSet::of(&[CardType::Creature]), AbilitiesOf::Sources);
    /// "Spend this mana only to cast artifact spells or activate abilities of artifacts" (Dalakos, Crafter of Wonders, Grand
    /// Architect, Power Depot, Vedalken Engineer, ...): artifact permanents' abilities only (CR 109.2).
    pub const ARTIFACT_SPELLS_OR_ABILITIES_OF_ARTIFACTS: ManaRestriction =
        ManaRestriction::SpellsOrAbilities(TypeSet::of(&[CardType::Artifact]), AbilitiesOf::Permanents);
    pub const SPELLS_FROM_EXILE: ManaRestriction = ManaRestriction::SpellsFrom(ZoneKind::Exile);
    pub const NOT_SPELLS_FROM_HAND: ManaRestriction = ManaRestriction::NotSpellsFrom(ZoneKind::Hand);

    /// "Spend this mana only to cast spells with mana value `n` or greater."
    pub const fn mana_value_at_least(n: u8) -> ManaRestriction {
        ManaRestriction::ManaValue(ManaValueSpells { at_least: n, or_x: false })
    }

    /// "Spend this mana only to cast spells with mana value `n` or greater or spells with {X} in their mana costs."
    pub const fn mana_value_at_least_or_x(n: u8) -> ManaRestriction {
        ManaRestriction::ManaValue(ManaValueSpells { at_least: n, or_x: true })
    }

    /// The restrictions with no subtype or number in them, each with the name a scenario and the dump write it by. Those
    /// of `Subtypes` are named for their subtypes and those of `ManaValue` for their number (`name`). A card naming a
    /// restriction of card types not here fails the build (`Adds::only_for` is checked at compile time), since the host
    /// has no word for it on the wire.
    pub const NAMED: [(ManaRestriction, &'static str); 18] = [
        (ManaRestriction::NotNonartifactSpells, "NotNonartifactSpells"),
        (ManaRestriction::GainsHaste, "GainsHaste"),
        (ManaRestriction::LegendaryUncounterable, "LegendarySpells, CantBeCountered"),
        (ManaRestriction::ARTIFACT_SPELLS, "ArtifactSpells"),
        (ManaRestriction::INSTANT_OR_SORCERY_SPELLS, "InstantOrSorcerySpells"),
        (ManaRestriction::PLANESWALKER_SPELLS, "PlaneswalkerSpells"),
        (ManaRestriction::ABILITIES, "Abilities"),
        (ManaRestriction::LAND_ABILITIES, "LandAbilities"),
        (ManaRestriction::ARTIFACT_ABILITIES, "ArtifactAbilities"),
        (ManaRestriction::NONCREATURE_SPELLS, "NoncreatureSpells"),
        (ManaRestriction::CREATURE_SPELLS, "CreatureSpells"),
        (ManaRestriction::CREATURE_ABILITIES, "CreatureAbilities"),
        (ManaRestriction::SPELLS_FROM_EXILE, "SpellsFromExile"),
        (ManaRestriction::NOT_SPELLS_FROM_HAND, "NotSpellsFromHand"),
        (ManaRestriction::ABILITIES_OF_ARTIFACTS, "AbilitiesOfArtifacts"),
        (ManaRestriction::CREATURE_SPELLS_OR_ABILITIES_OF_CREATURES, "CreatureSpellsOrAbilitiesOfCreatures"),
        (ManaRestriction::CREATURE_SPELLS_OR_ABILITIES, "CreatureSpellsOrAbilities"),
        (ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES_OF_ARTIFACTS, "ArtifactSpellsOrAbilitiesOfArtifacts"),
    ];

    const fn same(self, other: ManaRestriction) -> bool {
        match (self, other) {
            (ManaRestriction::NotNonartifactSpells, ManaRestriction::NotNonartifactSpells)
            | (ManaRestriction::GainsHaste, ManaRestriction::GainsHaste)
            | (ManaRestriction::LegendaryUncounterable, ManaRestriction::LegendaryUncounterable) => true,
            (ManaRestriction::Spells(a), ManaRestriction::Spells(b)) | (ManaRestriction::NonSpells(a), ManaRestriction::NonSpells(b)) => {
                a.0 == b.0
            }
            (ManaRestriction::Abilities(a, x), ManaRestriction::Abilities(b, y))
            | (ManaRestriction::SpellsOrAbilities(a, x), ManaRestriction::SpellsOrAbilities(b, y)) => a.0 == b.0 && x as u8 == y as u8,
            (ManaRestriction::SpellsFrom(a), ManaRestriction::SpellsFrom(b))
            | (ManaRestriction::NotSpellsFrom(a), ManaRestriction::NotSpellsFrom(b)) => a as u8 == b as u8,
            _ => false,
        }
    }

    /// Whether the wire has a word for it: one of `NAMED`, or any of `ManaValue` or `Subtypes` - but one of creature
    /// spells or abilities of creature permanents of a subtype, which no card prints.
    pub const fn is_known(self) -> bool {
        if let ManaRestriction::Subtypes(SubtypeSpells { creature: true, abilities: Some(AbilitiesOf::Permanents), .. }) = self {
            return false;
        }
        if let ManaRestriction::Subtypes(_) | ManaRestriction::ManaValue(_) = self {
            return true;
        }
        let mut i = 0;
        while i < ManaRestriction::NAMED.len() {
            if self.same(ManaRestriction::NAMED[i].0) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Whether it limits mana by the subtypes of the spell it pays for, which only then need working out.
    pub fn names_subtypes(self) -> bool {
        matches!(self, ManaRestriction::Subtypes(_))
    }

    /// The least mana value of a spell it pays for, where it limits mana by that (`ManaValue`): what a spell with {X}
    /// still to be announced could reach (advice).
    pub fn least_mana_value(self) -> Option<u8> {
        match self {
            ManaRestriction::ManaValue(m) => Some(m.at_least),
            _ => None,
        }
    }

    /// Whether mana under this restriction may be spent on `payment` (CR 106.6).
    pub fn may_pay(self, payment: &ManaPayment) -> bool {
        match (self, *payment) {
            (ManaRestriction::NonSpells(barred), ManaPayment::Spell { types, .. }) => barred.0 & types.0 == 0,
            (ManaRestriction::ManaValue(m), ManaPayment::Spell { mana_value, x, .. }) => m.admits(mana_value, x),
            // "Can't be spent to cast nonartifact spells": everything but casting a nonartifact spell.
            (ManaRestriction::NotNonartifactSpells, ManaPayment::Spell { types, .. }) => types.has(CardType::Artifact),
            (ManaRestriction::NotNonartifactSpells, _) => true,
            (ManaRestriction::SpellsFrom(zone), ManaPayment::Spell { from, .. }) => from == zone,
            // "Can't be spent to cast spells from your hand": everything but casting a spell from the hand.
            (ManaRestriction::NotSpellsFrom(zone), ManaPayment::Spell { from, .. }) => from != zone,
            (ManaRestriction::NotSpellsFrom(_), _) => true,
            (ManaRestriction::GainsHaste, _) => true,
            (ManaRestriction::LegendaryUncounterable, ManaPayment::Spell { legendary, .. }) => legendary,
            (ManaRestriction::Spells(allowed) | ManaRestriction::SpellsOrAbilities(allowed, _), ManaPayment::Spell { types, .. }) => {
                allowed.0 & types.0 != 0
            }
            (ManaRestriction::Abilities(allowed, of), ManaPayment::Ability { source, on_battlefield, .. }) => {
                of.admits(on_battlefield) && (allowed.0 == 0 || allowed.0 & source.0 != 0)
            }
            (ManaRestriction::SpellsOrAbilities(allowed, of), ManaPayment::Ability { source, on_battlefield, .. }) => {
                of.admits(on_battlefield) && allowed.0 & source.0 != 0
            }
            (ManaRestriction::Subtypes(s), ManaPayment::Spell { types, ref subtypes, .. }) => s.admits(types, subtypes),
            (ManaRestriction::Subtypes(s), ManaPayment::Ability { source, ref subtypes, on_battlefield }) => {
                s.abilities.is_some_and(|of| of.admits(on_battlefield))
                    && (!s.creature || source.has(CardType::Creature))
                    && s.subtypes.iter().flatten().any(|&t| subtypes.has(t))
            }
            _ => false,
        }
    }

    /// What spending this mana on a spell does to it (CR 106.6): `MANA_GIVES_HASTE`, `MANA_MAKES_UNCOUNTERABLE`.
    pub fn effects(self) -> u8 {
        match self {
            ManaRestriction::GainsHaste => MANA_GIVES_HASTE,
            ManaRestriction::LegendaryUncounterable => MANA_MAKES_UNCOUNTERABLE,
            ManaRestriction::Subtypes(s) if s.uncounterable => MANA_MAKES_UNCOUNTERABLE,
            _ => 0,
        }
    }

    /// The name a scenario's `Spend:` and the state dump write it by: the limit and the effect, as the C# engine does.
    /// One of `Subtypes` is its subtypes (spaces left out) and "Spells" ("DragonSpells", "DragonOrOmenSpells", "ElfCreatureSpells",
    /// "ElementalSpellsOrAbilities"),
    /// "NoTypeCreatureSpells" for a chosen type never chosen, with ", CantBeCountered" after it for its effect.
    pub fn name(self) -> String {
        if let ManaRestriction::ManaValue(m) = self {
            return format!("ManaValue{}OrGreater{}Spells", m.at_least, if m.or_x { "OrX" } else { "" });
        }
        if let ManaRestriction::Subtypes(s) = self {
            let types: Vec<String> = s.subtypes.iter().flatten().map(|t| t.name().replace(' ', "")).collect();
            let mut name = if types.is_empty() { "NoType".to_string() } else { types.join("Or") };
            if s.chosen {
                name = "ChosenType".to_string();
            }
            name.push_str(if s.creature { "CreatureSpells" } else { "Spells" });
            match s.abilities {
                Some(AbilitiesOf::Sources) => name.push_str("OrAbilities"),
                Some(AbilitiesOf::Permanents) => name.push_str("OrAbilitiesOfPermanents"),
                None => {}
            }
            if s.uncounterable {
                name.push_str(", CantBeCountered");
            }
            return name;
        }
        ManaRestriction::NAMED.iter().find(|(r, _)| *r == self).map_or_else(|| format!("{self:?}"), |(_, n)| n.to_string())
    }

    /// Reads the words of a `Spend:` piece after the mana symbol: each a name of the limit or the effect.
    pub fn parse<'a>(words: impl Iterator<Item = &'a str>) -> Option<ManaRestriction> {
        let text: Vec<String> = words.map(|w| w.trim_matches(',').to_ascii_lowercase()).filter(|w| !w.is_empty()).collect();
        let matches = |name: &str| {
            let own: Vec<String> = name.split(',').map(|w| w.trim().to_ascii_lowercase()).collect();
            own.len() == text.len() && own.iter().all(|w| text.contains(w))
        };
        if let Some((r, _)) = ManaRestriction::NAMED.iter().find(|(_, n)| matches(n)) {
            return Some(*r);
        }
        // A mana value restriction: "ManaValue4OrGreaterSpells", "ManaValue5OrGreaterOrXSpells".
        if let [one] = text.as_slice()
            && let Some(rest) = one.strip_prefix("manavalue")
        {
            let (n, or_x) = match rest.strip_suffix("orgreaterorxspells") {
                Some(n) => (n, true),
                None => (rest.strip_suffix("orgreaterspells")?, false),
            };
            return Some(ManaRestriction::ManaValue(ManaValueSpells { at_least: n.parse().ok()?, or_x }));
        }
        // A subtype restriction: "DragonSpells", "DragonOrOmenSpells", "ElfCreatureSpells", with "CantBeCountered".
        let uncounterable = text.iter().any(|w| w == "cantbecountered");
        if text.len() != 1 + uncounterable as usize {
            return None;
        }
        let limit = text.iter().find(|w| *w != "cantbecountered")?;
        let (limit, abilities) = match (limit.strip_suffix("orabilities"), limit.strip_suffix("orabilitiesofpermanents")) {
            (Some(rest), _) => (rest, Some(AbilitiesOf::Sources)),
            (None, Some(rest)) => (rest, Some(AbilitiesOf::Permanents)),
            (None, None) => (limit.as_str(), None),
        };
        let (body, creature) = match limit.strip_suffix("creaturespells") {
            Some(body) => (body, true),
            None => (limit.strip_suffix("spells")?, false),
        };
        // One subtype's name, or two joined by "Or": read against the names themselves, since a name may hold "or" (Orc,
        // Warrior, Horror).
        let named = |text: &str| Subtype::ALL.into_iter().find(|t| t.name().replace(' ', "").eq_ignore_ascii_case(text));
        let subtypes = if body == "notype" {
            [None, None]
        } else if let Some(one) = named(body) {
            [Some(one), None]
        } else {
            Subtype::ALL.into_iter().find_map(|first| {
                let rest = body.strip_prefix(first.name().replace(' ', "").to_ascii_lowercase().as_str())?.strip_prefix("or")?;
                Some([Some(first), Some(named(rest)?)])
            })?
        };
        Some(ManaRestriction::Subtypes(SubtypeSpells { subtypes, chosen: false, creature, uncounterable, abilities }))
    }
}

impl Player {
    /// A player as the game begins (CR 103.4): `life` life, and nothing else.
    pub fn starting(life: i32) -> Player {
        Player { life, starting_life: life, ..Player::default() }
    }

    /// Poison counters (CR 122.1f).
    pub fn poison(&self) -> u8 {
        self.poison
    }

    /// Energy counters (CR 107.14).
    pub fn energy(&self) -> u8 {
        self.energy
    }

    /// Experience counters (CR 122.1).
    pub fn experience(&self) -> u8 {
        self.experience
    }

    /// Pays `n` energy (CR 107.14): energy counters taken off, never below none. Not putting counters.
    pub fn pay_energy(&mut self, n: u8) {
        self.energy = self.energy.saturating_sub(n);
    }

    /// Takes every counter off the player: a normal form, for comparing positions (`cantrip_rules::loops`).
    pub fn clear_counters(&mut self) {
        (self.poison, self.energy, self.experience) = (0, 0, 0);
    }

    /// The write of the put path for a player's counters: only `cantrip_rules::counters` calls this (see
    /// `Obj::put_path_add`). Each kind stops at 255.
    #[doc(hidden)]
    pub fn put_path_add(&mut self, poison: u16, energy: u16, experience: u16) {
        let add = |n: u8, more: u16| (n as u32 + more as u32).min(255) as u8;
        self.poison = add(self.poison, poison);
        self.energy = add(self.energy, energy);
        self.experience = add(self.experience, experience);
    }

    /// Spends `n` mana of one kind from the pool, the ordinary mana of that kind before the mana that lasts.
    /// The mana in the pool that lasts until the end of combat.
    pub fn lasting_mana(&self) -> ManaPool {
        let mut lasting = self.lasting;
        for (l, p) in lasting.0.iter_mut().zip(self.pool.0) {
            *l = (*l).min(p);
        }
        lasting
    }

    pub fn spend_mana(&mut self, mana: Mana, n: u8) {
        let at = mana as usize;
        self.pool.0[at] -= u32::from(n);
        self.lasting.0[at] = self.lasting.0[at].min(self.pool.0[at]);
    }
}

/// Where the turn is (CR 500).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Turn {
    pub number: u32,
    pub active: PlayerId,
    pub step_raw: u8,
    pub priority: PlayerId,
    /// Players who have passed in succession with nothing new on the stack (CR 117.4).
    pub passes: u8,
    /// Whether the step's turn-based actions have been done.
    pub began: bool,
    /// The player who took the first turn of the game, who skips their first draw (CR 103.8a).
    pub starting: PlayerId,
    /// Whether combat damage has been dealt this combat; whether attackers were declared at all.
    pub attacked: bool,
    /// Combat damage steps done this combat: 1 after a first-strike step (CR 510.4), 2 after the regular one.
    pub damage_round: u8,
    /// The number of the turn "damage can't be prevented this turn" was made (CR 615.12), 0 for never.
    pub no_prevention_turn: u32,
    /// The number of the turn "creatures can't block this turn" was made (CR 509.1b, 611.2c), 0 for never: it reaches
    /// every creature, those that arrive later included.
    pub no_blocks_turn: u32,
    /// Day or night (CR 731.1): a fact about the game, kept here with the other facts about its turns.
    pub day_night: DayNight,
    /// The player who took the previous turn (CR 502.2, 731.2): whose spells the untap step counts.
    pub previous: PlayerId,
    /// Additional combat phases still to come this turn (CR 500.8): each follows the combat phase it was created in, or the
    /// precombat main phase's, as the end of combat step ends.
    pub extra_combats: u32,
    /// The player whose turn in the ordinary rotation was taken last, while extra turns are being taken (CR 500.7: an
    /// extra turn is taken between turns and does not move the rotation); none while no extra turn has been taken.
    pub rotation: Option<PlayerId>,
    /// An effect has ended the turn (CR 724.1d): the step under way ends with no player given priority again, and the
    /// game skips straight to the cleanup step.
    pub ending: bool,
    /// Players were given priority in this cleanup step (CR 514.3a), so another cleanup step follows it.
    pub cleanup_again: bool,
    /// The extra turn this turn is (CR 500.7), by its `ExtraTurn::id`: what "that turn" names in an ability that created it
    /// (Final Fortune's "at the beginning of that turn's end step"). 0 for a turn of the ordinary rotation.
    pub extra: u32,
}

/// An extra turn waiting to be taken (CR 500.7): whose it is, and the number that names it, unique in the game, by which an
/// effect that created it refers to "that turn" (`DelayedWhen::ExtraTurnStep`, `Turn::extra`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtraTurn {
    pub player: PlayerId,
    pub id: u32,
}

impl Turn {
    pub fn step(&self) -> Step {
        // Safety of the cast is the enum's own range: steps are only ever written through `set_step`.
        match self.step_raw {
            0 => Step::Untap,
            1 => Step::Upkeep,
            2 => Step::Draw,
            3 => Step::PrecombatMain,
            4 => Step::BeginCombat,
            5 => Step::DeclareAttackers,
            6 => Step::DeclareBlockers,
            7 => Step::CombatDamage,
            8 => Step::EndOfCombat,
            9 => Step::PostcombatMain,
            10 => Step::End,
            _ => Step::Cleanup,
        }
    }

    pub fn set_step(&mut self, step: Step) {
        self.step_raw = step as u8;
    }
}

/// An attacking creature and what it attacks (CR 506.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Attacker {
    pub id: ObjectId,
    /// The player, planeswalker or battle it attacks; `Target::None` once that planeswalker or battle has been removed from
    /// combat (CR 506.4, 506.4c): it is still an attacking creature, attacking nothing, and may be blocked, but unblocked it
    /// assigns no combat damage (CR 510.1b). Cleared as the removal happens (`ops::remove_from_combat`,
    /// `ops::settle_combat`), so a permanent named here is one still being attacked.
    pub target: Target,
    /// Its defending player (CR 508.5): the player it attacks, or the controller of the planeswalker or the protector of the
    /// battle it attacks - fixed while it attacks it, as a change of either removes that permanent from combat (CR 506.4) -
    /// and still that player once the planeswalker or battle has been removed (`target` `Target::None`).
    pub defending: PlayerId,
    /// What the planeswalker or battle it attacks is being attacked as (CR 506.4, 506.4e): of its card types, planeswalker,
    /// battle or both - what it was as it was attacked, kept up to date while it stays attacked (`ops::settle_combat`), so
    /// a change from both to one is told from a change from one to the other. None for a player, or once it attacks nothing.
    pub attacked_as: TypeSet,
    pub blocked: bool,
    /// It had first strike or double strike as this combat's first-strike damage step began (CR 510.4): in the regular
    /// combat damage step it assigns combat damage only if it has double strike then.
    pub first_strike: bool,
    /// The attacking band it is in (CR 702.22c), numbered from 1 in the order they were announced; 0 for none. It lasts the
    /// rest of combat whatever happens to banding (CR 702.22e), and a member removed from combat leaves it (CR 702.22f).
    pub band: u32,
}

impl Attacker {
    /// An attacking creature attacking `target` as `attacked_as`, whose defending player is `defending`: unblocked, in no
    /// band, not yet in any combat damage step.
    pub const fn new(id: ObjectId, target: Target, defending: PlayerId, attacked_as: TypeSet) -> Attacker {
        Attacker { id, target, defending, attacked_as, blocked: false, first_strike: false, band: 0 }
    }
}

/// A blocking creature and the attacker it blocks (CR 509.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Block {
    pub blocker: ObjectId,
    pub attacker: ObjectId,
    /// The blocker had first strike or double strike as this combat's first-strike damage step began (CR 510.4).
    pub first_strike: bool,
}

/// How the game ended, if it has (CR 104).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Outcome {
    #[default]
    InProgress,
    Won(PlayerId),
    Draw,
}

/// The shuffler and roller, in the state so that a game replays exactly (PCG32).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rng {
    pub state: u64,
    pub inc: u64,
    /// Die rolls a script has fixed in advance, five bits each (the face, 1 to 20), the next in the low bits: a scenario
    /// has to be able to say what a die shows. Used up before the generator is.
    pub forced: u64,
    pub forced_len: u8,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut rng = Rng { state: 0, inc: (seed << 1) | 1, forced: 0, forced_len: 0 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed ^ 0x853c_49e6_748f_ea9b);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// The next die roll, 1 to `sides` (CR 706.2): the next face fixed for the game, if any, else a random one.
    pub fn roll(&mut self, sides: u32) -> u32 {
        if self.forced_len > 0 {
            let face = (self.forced & 0b11111) as u32;
            self.forced >>= 5;
            self.forced_len -= 1;
            return face.clamp(1, sides.max(1));
        }
        self.below(sides) + 1
    }

    /// Uniform in `0..bound`, without modulo bias.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound <= 1 {
            return 0;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "The chosen creature" refers to the object chosen as the permanent entered only from an ability linked to the one that
    /// chose it (CR 607.2d, 607.5): of the same text, which the permanent has as its own. One of another text, or one it
    /// acquired by a copy effect or a grant since, even of the same card, refers to nothing.
    #[test]
    fn a_choice_is_read_only_by_an_ability_linked_to_the_one_that_made_it() {
        let chosen = Chosen { value: ObjectId(9), text: CardId(4), acquired: Acquired::OWN };
        let by = |text, acquired| Linked { object: ObjectId(2), text, acquired };
        let copy = Acquired { by: AcquiredBy::Copy, stamp: 7, part: 0 };
        assert_eq!(chosen.to(by(CardId(4), Acquired::OWN)), Some(ObjectId(9)));
        assert_eq!(chosen.to(by(CardId(5), Acquired::OWN)), None);
        assert_eq!(chosen.to(by(CardId(4), copy)), None);
        assert_eq!(chosen.to(by(CardId(4), Acquired { by: AcquiredBy::Grant, stamp: 7, part: 1 })), None);
        // A choice an ability a copy effect gave it made is read by that copy effect's abilities alone (CR 607.5).
        let by_copy = Chosen { value: CardId(6), text: CardId(4), acquired: copy };
        assert_eq!(by_copy.to(by(CardId(4), copy)), Some(CardId(6)));
        assert_eq!(by_copy.to(by(CardId(4), Acquired::OWN)), None);
    }

    /// Each ability that chose a name keeps its own (CR 607.2d): a second ability's name doesn't take the first's place, the
    /// same ability choosing again replaces its own, and with every place taken (`CHOSEN_NAMES`) a new one is kept no more.
    #[test]
    fn each_ability_keeps_the_name_it_chose() {
        let entered = <Choices as ChoiceRecord>::entered(CardId(4), Entered { card: Some(CardId(10)), ..Default::default() });
        let own = Linked { object: ObjectId(2), text: CardId(4), acquired: Acquired::OWN };
        let given = Linked { object: ObjectId(2), text: CardId(5), acquired: Acquired { by: AcquiredBy::Grant, stamp: 3, part: 0 } };
        let other = Linked { object: ObjectId(2), text: CardId(6), acquired: Acquired::OWN };
        let mut choices = entered;
        assert_eq!(choices.card(own), Some(CardId(10)), "the name chosen as it entered");
        assert!(choices.name(given, CardId(11)));
        assert_eq!((choices.card(own), choices.card(given)), (Some(CardId(10)), Some(CardId(11))), "each its own");
        assert!(choices.name(given, CardId(12)));
        assert_eq!((choices.card(own), choices.card(given)), (Some(CardId(10)), Some(CardId(12))), "the same ability's replaced");
        assert!(!choices.name(other, CardId(13)), "every place taken");
        assert_eq!((choices.card(own), choices.card(given), choices.card(other)), (Some(CardId(10)), Some(CardId(12)), None));
    }

    /// A subtype restriction's name reads back as the same restriction, whatever its subtypes hold: "or" inside a name
    /// (Orc, Warrior, Horror) and spaces (Time Lord) included.
    #[test]
    fn subtype_restriction_names_read_back() {
        let one = |t| ManaRestriction::Subtypes(SubtypeSpells::of(t));
        let mut all: Vec<ManaRestriction> = Subtype::ALL.into_iter().map(one).collect();
        all.push(ManaRestriction::Subtypes(SubtypeSpells::either(Subtype::Orc, Subtype::Warrior)));
        all.push(ManaRestriction::Subtypes(SubtypeSpells::either(Subtype::Horror, Subtype::Orc)));
        all.push(ManaRestriction::Subtypes(SubtypeSpells::either(Subtype::Dragon, Subtype::Omen)));
        all.push(ManaRestriction::Subtypes(SubtypeSpells { subtypes: [Some(Subtype::Elf), None], chosen: false, creature: true, uncounterable: true, abilities: None }));
        all.push(ManaRestriction::Subtypes(SubtypeSpells { subtypes: [None, None], chosen: false, creature: true, uncounterable: false, abilities: None }));
        all.push(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elemental).or_abilities()));
        all.push(ManaRestriction::Subtypes(SubtypeSpells { subtypes: [Some(Subtype::Elf), None], chosen: false, creature: true, uncounterable: false, abilities: Some(AbilitiesOf::Sources) }));
        all.push(ManaRestriction::Subtypes(SubtypeSpells { subtypes: [None, None], chosen: false, creature: false, uncounterable: false, abilities: Some(AbilitiesOf::Sources) }));
        all.push(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elemental).or_abilities_of_permanents()));
        all.push(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Myr).or_abilities_of_permanents()));
        all.push(ManaRestriction::mana_value_at_least(4));
        all.push(ManaRestriction::mana_value_at_least_or_x(5));
        all.push(ManaRestriction::mana_value_at_least(12));
        all.extend(ManaRestriction::NAMED.map(|(r, _)| r));
        for r in all {
            let name = r.name();
            assert_eq!(ManaRestriction::parse(name.split_whitespace()), Some(r), "{name}");
        }
    }

    /// "Abilities of [type]s" are a permanent's only (CR 109.2), "abilities of [type] sources" a source's in any zone (CR
    /// 109.2c): a creature card's cycling in the hand is paid by Gwenna's mana, not by Castle Garenbrig's (its ruling), nor
    /// Soldevi Machinist's by an artifact card's unearth, nor Smokebraider's by an Elemental card's.
    #[test]
    fn abilities_of_a_type_are_a_permanents_and_of_a_type_source_any_sources() {
        let creature = TypeSet::of(&[CardType::Creature]);
        let artifact = TypeSet::of(&[CardType::Artifact]);
        let elemental = || SubtypeSet::of(&[Subtype::Elemental]);
        let ability = |source, subtypes, on_battlefield| ManaPayment::Ability { source, subtypes, on_battlefield };
        let spell = |types| ManaPayment::Spell { types, subtypes: SubtypeSet::NONE, legendary: false, mana_value: 2, x: false, from: ZoneKind::Hand };
        let castle = ManaRestriction::CREATURE_SPELLS_OR_ABILITIES_OF_CREATURES;
        assert!(castle.may_pay(&spell(creature)));
        assert!(!castle.may_pay(&spell(artifact)));
        assert!(castle.may_pay(&ability(creature, SubtypeSet::NONE, true)));
        assert!(!castle.may_pay(&ability(creature, SubtypeSet::NONE, false)));
        assert!(!castle.may_pay(&ability(artifact, SubtypeSet::NONE, true)));
        assert!(!castle.may_pay(&ManaPayment::Other));
        let gwenna = ManaRestriction::CREATURE_SPELLS_OR_ABILITIES;
        assert!(gwenna.may_pay(&ability(creature, SubtypeSet::NONE, false)));
        assert!(!gwenna.may_pay(&ability(artifact, SubtypeSet::NONE, false)));
        assert!(gwenna.may_pay(&spell(creature)));
        let artifacts = ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES_OF_ARTIFACTS;
        assert!(artifacts.may_pay(&spell(artifact)) && artifacts.may_pay(&ability(artifact, SubtypeSet::NONE, true)));
        assert!(!artifacts.may_pay(&ability(artifact, SubtypeSet::NONE, false)) && !artifacts.may_pay(&spell(creature)));
        let soldevi = ManaRestriction::ABILITIES_OF_ARTIFACTS;
        assert!(soldevi.may_pay(&ability(artifact, SubtypeSet::NONE, true)));
        assert!(!soldevi.may_pay(&ability(artifact, SubtypeSet::NONE, false)) && !soldevi.may_pay(&spell(artifact)));
        assert!(ManaRestriction::ARTIFACT_ABILITIES.may_pay(&ability(artifact, SubtypeSet::NONE, false)));
        assert!(ManaRestriction::ABILITIES.may_pay(&ability(TypeSet::NONE, SubtypeSet::NONE, false)));
        let smokebraider = ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elemental).or_abilities_of_permanents());
        assert!(smokebraider.may_pay(&ability(creature, elemental(), true)));
        assert!(!smokebraider.may_pay(&ability(creature, elemental(), false)));
        let flamebraider = ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Elemental).or_abilities());
        assert!(flamebraider.may_pay(&ability(creature, elemental(), false)));
    }
}

#[cfg(test)]
mod copiable_tests {
    use super::*;

    /// Every `Obj` holds an `Option<Copiable>`, so a copy's exceptions are packed: the yes-or-no ones and a stated loyalty
    /// take three bytes, and an object pays nothing for them. Each reads back as it was set, a later one over an earlier.
    #[test]
    fn the_rare_exceptions_cost_an_object_nothing_and_read_back() {
        assert!(std::mem::size_of::<Option<Copiable>>() <= 160, "Option<Copiable> is {} bytes", std::mem::size_of::<Option<Copiable>>());
        let plain = Copiable::of(CardId(1));
        assert!(!plain.has_exceptions() && plain.legendary_stated().is_none() && plain.stated_loyalty().is_none());
        let ob = plain.nonlegendary().with_loyalty(0);
        assert!(ob.has_exceptions() && !ob.types_stated() && !ob.has_no_cost());
        assert_eq!((ob.legendary_stated(), ob.stated_loyalty()), (Some(false), Some(0)));
        let again = ob.legendary().with_loyalty(-3).costless();
        assert_eq!((again.legendary_stated(), again.stated_loyalty(), again.has_no_cost()), (Some(true), Some(-3), true));
        assert_eq!(again.nonlegendary().legendary_stated(), Some(false));
        // A stated power and toughness, and the faces of a double-faced object (no exception, CR 707.8a), read back.
        assert_eq!((plain.pt(), plain.sized(4, -1).pt()), (None, Some((4, -1))));
        let faces = plain.from_faces(Some(CardId(7)), true);
        assert!(!faces.has_exceptions() && faces.back_up() && !faces.is_own_faces());
        assert_eq!((faces.faces(), faces.from_faces(None, true).faces()), (Some(CardId(7)), None));
        assert!(plain.own_faces_of(CardId(7)).is_own_faces() && !plain.own_faces_of(CardId(7)).has_exceptions());
        assert!(!plain.own_faces_of(CardId(7)).from_faces(Some(CardId(7)), false).is_own_faces());
    }

    /// A seventh distinct ability added by exceptions to a copy that holds six: not taken, the six and their counts left as
    /// they were, and no panic (it panicked).
    #[test]
    fn a_seventh_exception_ability_is_not_taken_and_nothing_panics() {
        let ability = |index: u8| AbilityRef { card: CardId(7), index };
        let mut six = Copiable::of(CardId(1));
        for index in 0..Copiable::EXTRA_ABILITIES as u8 {
            six = six.with_ability(ability(index));
        }
        six = six.with_ability(ability(0));
        let seventh = six.with_ability(ability(Copiable::EXTRA_ABILITIES as u8));
        assert_eq!(seventh, six, "a full list takes no more");
        assert_eq!(seventh.extra_instances().count(), Copiable::EXTRA_ABILITIES + 1);
    }
}
