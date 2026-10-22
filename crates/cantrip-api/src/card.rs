//! What a card is: its characteristics, and its behavior as code, reached by function pointer. Nothing about a card is
//! read from data at runtime, and nothing is parsed.

use cantrip_state::*;

use crate::chars::Chars;
use crate::event::{Event, EventMask};
use crate::history::History;
use crate::game::{CastTerms, CastWay, Game, Query, Suspend};
use crate::replace::{GrantedPreventionDef, GrantedReplacementDef, PreventionDef, ReplacementDef};

/// A spell or ability's effect as it resolves (CR 608.2).
pub type Effect = fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>;

/// Whether a target is legal for a target slot (CR 115).
pub type TargetFilter = fn(&dyn Query, &Asker, Target) -> bool;

/// Whether an event triggers an ability of `source` (CR 603.2), and what the ability should carry of it.
pub type TriggerMatch = fn(&dyn Query, ObjectId, &Event) -> Option<Fired>;

/// Whether an event triggers an ability (CR 603.2), asked with the ability's source and its controller: a trigger event
/// that says "you" reads `controller`. For an ability of a permanent that is the permanent's controller; for a delayed
/// triggered ability it is the player who controlled the spell or ability that created it (CR 603.7d-e), whoever controls
/// its source now and wherever the source has gone. Any matcher a delayed ability can use that says "you" is written so.
pub type AskedMatch = fn(&dyn Query, &Asker, &Event) -> Option<Fired>;

/// What a triggered ability carries of the event that triggered it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Fired {
    /// The player or object the event was about: "that player", "that card". Not a target.
    pub subject: Target,
    pub data: [u32; 2],
}

impl Fired {
    pub const PLAIN: Option<Fired> = Some(Fired { subject: Target::None, data: [0; 2] });

    pub fn about(subject: Target) -> Option<Fired> {
        Some(Fired { subject, data: [0; 2] })
    }
}

/// An intervening "if" (CR 603.4): checked as the ability triggers and again as it resolves.
pub type TriggerCondition = fn(&dyn Query, &Asker) -> bool;

/// "As [this] enters" and "[this] enters with" (CR 614.1c-d, 614.12): applied to the object about to enter, which
/// is not on the battlefield yet.
pub type Enters = fn(&mut dyn Game, &mut Entering) -> Result<(), Suspend>;

/// "As this [permanent] transforms (into [this face]), ..." (CR 712.20): "If a double-faced card would have an 'As [this
/// permanent] transforms . . .' ability after it transforms or converts, that ability is applied while that permanent is
/// transforming or converting, not afterward." A static ability of the face the permanent turns over to, read as it
/// would be once turned over - so it is that face's (`CardDef::as_transforms` of the back face for "transforms into" the
/// back), and a permanent that would have lost all its abilities there has none (CR 613.1f) - and applied by the core as
/// part of the transformation itself, whatever spell, ability or rule made it transform (`Game::transform`,
/// `transform_by`, day and night). Before the `Event::Transformed` it is part of, so "whenever this transforms" sees what
/// it did. A permanent put onto the battlefield transformed did not transform (CR 712.14a; Sephiroth's ruling): it is
/// not applied then.
///
/// `asker` is the permanent and its controller, the ability's "you". The ability may ask (`Game::ask`): the effect that
/// made it transform runs again from its start once answered, as every question does. Sephiroth, One-Winged Angel's "you
/// get an emblem", Shinryu, Transcendent Rival's "choose an opponent", Curse of Leeches' "attach it to a player", and
/// Olag, Ludevic's Hubris's "it becomes a copy of a creature card exiled with it" print it.
pub type AsTransforms = fn(&mut dyn Game, &Asker) -> Result<(), Suspend>;

/// A permanent about to enter the battlefield, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entering {
    pub object: ObjectId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub counters: [i16; CounterKind::COUNT],
    pub copy: Option<Copiable>,
    /// It enters prepared (CR 722.2).
    pub prepared: bool,
    /// The color or type chosen as it enters (CR 614.12), kept on the permanent.
    pub chosen_color: Option<Color>,
    pub chosen_subtype: Option<Subtype>,
    /// The player chosen as it enters (CR 614.12a): True-Name Nemesis.
    pub chosen_player: Option<PlayerId>,
    /// The card name chosen as it enters (CR 614.12a).
    pub chosen_card: Option<CardId>,
    /// One of the words its text offers to choose between, chosen as it enters (CR 614.12a, 607.2f): Gollum, Riddle
    /// Master's odd or even, a Siege's "Khans or Dragons" (`cantrip_mechanics::choose::option_as_enters`).
    pub chosen_option: Option<ChosenOption>,
    /// It enters attached to this permanent (CR 303.4f): an Aura entering from the stack, or choosing its host as it
    /// is put onto the battlefield (CR 303.4g).
    pub attach: Option<ObjectId>,
    /// It enters enchanting this player (CR 303.4f): an Aura with "enchant player" entering from the stack.
    pub attach_player: Option<PlayerId>,
    /// The X the spell it was cast as announced (CR 107.3m), which "enters with X counters" reads.
    pub x: u16,
    /// It enters with its back face up (CR 712.14a): its entering replacements and loyalty are the back face's.
    pub transformed: bool,
    /// The player chosen to protect a battle as it enters (CR 310.9a, 310.12a), by `prepare_entry`.
    pub protector: Option<PlayerId>,
    /// It does not enter: it goes to this zone instead (CR 614.12, 614.1a), as Mox Diamond's replacement sends it to the graveyard
    /// when its controller does not discard a land.
    pub instead: Option<Zone>,
}

/// Where a spell countered by `Game::counter_to` goes instead of its owner's graveyard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterTo {
    Hand,
    LibraryTop,
    LibraryBottom,
    Exile,
}

/// Where cards put into their owners' libraries go (CR 401.4), for `Game::put_into_library`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryPlace {
    /// On top, the first of the cards given topmost.
    Top,
    /// On the bottom, the cards given in order, the last of them lowest.
    Bottom,
    /// On the bottom in a random order, drawn from the game's own random source (CR 401.4).
    BottomInRandomOrder,
    /// Into the library, which is then shuffled. The referenced owners' libraries are shuffled even when a referenced
    /// object has left its expected zone or a replacement prevents it from arriving (CR 701.24c).
    ShuffledIn,
    /// This many cards down from the top (0 is the top): "second from the top" is `FromTop(1)`. The cards given go in
    /// order, the first highest.
    FromTop(usize),
}

/// The layers and sublayers a static ability's effect applies in (CR 613.1, 613.4), in order. Copy effects (layer 1)
/// are the object's copiable values; control (2) is the object's controller itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// Layer 2: control-changing effects (CR 613.1b). A static ability of this layer (`control_changing`) gives its
    /// source's controller control of what it affects; it changes no characteristic, so the layers that work out
    /// characteristics pass it by, and the core settles each permanent's controller from it and the resolved
    /// control-changing effects in timestamp order (CR 613.7).
    Control,
    /// Layer 3: text-changing effects (CR 612), on the type line here; card code reads its rules text's words through
    /// `Query::words`.
    Text,
    /// Layer 4: card types, subtypes, supertypes.
    Type,
    /// Layer 5: colors.
    Color,
    /// Layer 6: abilities, keywords, protection, attack requirements.
    Ability,
    /// Layer 7a: characteristic-defining abilities that define power and toughness (CR 604.3).
    PtDefine,
    /// Layer 7b: power and toughness set to a value.
    PtSet,
    /// Layer 7c: power and toughness modified, counters included.
    PtModify,
    /// Layer 7d: power and toughness switched (CR 613.4d), after everything that sets or modifies them.
    PtSwitch,
    /// Past every layer: the finished characteristics.
    End,
}

/// What a static ability's functions may read while the layers are being applied: the state, the event history, and
/// other objects' characteristics as the layers before the one being applied leave them. Reading only earlier layers keeps
/// one function finite; the order effects apply in, dependencies included (CR 613.8), is the layer pass's
/// (`cantrip_rules::layers`), not the function's.
pub trait LayerQuery {
    fn state(&self) -> &GameState;
    fn cards(&self) -> &CardTable;
    /// Every event since the game began (`crate::history`): what a static ability's condition about the past asks, "as
    /// long as you've lost life this turn" (CR 611.3a: its effect is not locked in, so the layers ask it again whenever
    /// characteristics are computed, and the answer follows each event as it is recorded and each turn as it begins).
    fn history(&self) -> History<'_>;
    /// Another object's characteristics, through the layers before the one being applied.
    fn chars(&self, id: ObjectId) -> Chars;
    /// Another object's characteristics with its own characteristic-defining abilities of power and toughness applied too
    /// (layer 7a): what a characteristic-defining ability that reads the power of other objects needs, since those abilities
    /// apply first (CR 613.8a dependency). Mastercraft Raptor and Wretched Bonemass read the cards used to craft them, a
    /// Tarmogoyf among them counting as it is in exile (their rulings). Only for an object that is not the one being
    /// computed and whose own abilities do not read it back.
    fn chars_with_defined_pt(&self, id: ObjectId) -> Chars;
    /// An object's mana value (CR 202.3) from `so_far`, its characteristics as the layers have them now: its mana cost is
    /// settled with its copiable values (CR 613.2c), so this is what `Query::mana_value` gives, without asking for the
    /// characteristics still being worked out. What "its mana value" reads in a static ability's effect (Opalescence).
    fn mana_value(&self, id: ObjectId, so_far: &Chars) -> u32 {
        crate::mana_value_of(&|card| self.cards().get(card), self.state(), id, so_far)
    }
    /// The words an object's text has now (CR 612): what a static ability names, read through its source's words.
    fn words(&self, id: ObjectId) -> Words {
        crate::words(self.state(), id)
    }
    /// Who controls an object as the layers stand (CR 613.1b): in layer 2, as the control-changing effects applied so far
    /// leave it - what a control-changing ability whose reach depends on control reads ("each creature an opponent
    /// controls") - and after it, the controller the state holds, which layer 2 settled.
    fn controller(&self, id: ObjectId) -> PlayerId {
        self.state().obj(id).controller
    }
}

/// Whether a static ability applies to `affected`, given what the earlier layers made it (`so_far`).
pub type StaticAffects = fn(&dyn LayerQuery, ObjectId, ObjectId, &Chars) -> bool;

/// A static ability's effect on one affected object's characteristics.
///
/// The contract of every `apply` and `apply_each`: the `Chars` it is given is only written, and read only to guard (a land's
/// subtypes set only if it is a land, `Chars::set_land_subtypes`; an ability given only if not there yet). A value the
/// effect works out from its own object - "+X/+0, where X is its power", "its mana value" - is read through the view
/// (`LayerQuery::chars`, `LayerQuery::mana_value`), as the layer stands. The dependency system judges what an effect
/// does on the object as it is (CR 613.8a, 613.9), and only a read through the view shows it what changed it; a value
/// taken from `Chars` would hide the dependency. `CANTRIP_VERIFY` checks the contract (`layers::takes_from_its_object`).
pub type StaticApply = fn(&dyn LayerQuery, ObjectId, &mut Chars);

/// A static ability's effect on one affected object's characteristics that reads that object (`StaticDef::apply_each`):
/// the view, the source, the affected object, and its characteristics so far. The contract of `StaticApply` holds.
pub type StaticApplyEach = fn(&dyn LayerQuery, ObjectId, ObjectId, &mut Chars);

/// A static ability (CR 604) that generates a continuous effect on characteristics (CR 611.3, 613): "lands are
/// Swamps", "all cards are blue", "other Elves get +1/+1". It applies while its source is on the battlefield, to
/// every object `affects` accepts: permanents, or objects anywhere with `everywhere` (CR 611.3a). A
/// characteristic-defining ability (`own`, CR 604.3) applies only to its own object, and in every zone.
#[derive(Clone, Copy)]
pub struct StaticDef {
    pub text: &'static str,
    pub layer: Layer,
    pub affects: StaticAffects,
    pub apply: StaticApply,
    /// In place of `apply`, an effect that differs from object to object by what each is ("has base power and base
    /// toughness each equal to its mana value": Opalescence, March of the Machines): it is told the affected object
    /// (`static_ability_for_each`).
    pub apply_each: Option<StaticApplyEach>,
    pub everywhere: bool,
    pub own: bool,
    /// The zone its source functions from (CR 113.6b): the battlefield, the graveyard for Anger's kind, or exile for
    /// cipher's (CR 702.99a).
    pub from: ZoneKind,
    /// For a characteristic-defining ability of power and toughness printed as stars: what the stars are worth once the
    /// ability is lost (CR 208.2a, 208.5, 613.1f), as `*/1+*` is 0/1. The card's printed power and toughness stay unset.
    pub lost_pt: Option<(i16, i16)>,
    /// It makes what it affects lose all abilities (CR 613.1f, layer 6): `loses_all_abilities`. The layers read it to
    /// know which objects have lost their printed abilities to a static ability, so that every rule judged by the
    /// abilities a card prints (static abilities of layer 6 on, cost changes, replacement effects, play rules, trigger
    /// rules) agrees with what layer 6 did (`Query::lost_all_abilities`).
    pub loses_all: bool,
    /// It is a later part of the same effect as the static ability before it in the card's list (CR 613.6): one
    /// ability that applies in several layers ("each other non-Aura enchantment is a creature ... and has base power and
    /// base toughness each equal to its mana value", layers 4 and 7b) is written as one `StaticDef` per layer, the first
    /// part as itself and each later one marked `continues()`. A later part applies to the same set of objects the first
    /// part applied to as the object's characteristics were worked out, whatever its own `affects` would say by then, and
    /// it goes on applying though its source loses its abilities in a layer between them (`layers.rs`).
    pub continues: bool,
    /// It sets the land subtypes of what it affects (CR 305.7, layer 4): `sets_land_subtypes`. The layers read it to know
    /// which lands have lost the abilities of their rules text, from layer 4 on, so that their static abilities from layer
    /// 4 on (CR 613.8a: they depend on it), replacement effects, cost changes and other printed rules agree with it.
    pub sets_land_subtypes: bool,
}

fn affects_nothing(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, _: &Chars) -> bool {
    false
}

fn apply_nothing(_: &dyn LayerQuery, _: ObjectId, _: &mut Chars) {}

impl StaticDef {
    pub const BLANK: StaticDef =
        StaticDef {
            text: "",
            layer: Layer::Ability,
            affects: affects_nothing,
            apply: apply_nothing,
            apply_each: None,
            everywhere: false,
            own: false,
            from: ZoneKind::Battlefield,
            lost_pt: None,
            loses_all: false,
            continues: false,
            sets_land_subtypes: false,
        };
}

/// What a cost is being worked out for (CR 601.2f): a spell about to be cast, or an ability about to be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Costed {
    /// The card that is being cast.
    Spell(ObjectId),
    /// The permanent (or card) whose ability is being activated, and which of its abilities.
    Ability(ObjectId, AbilityRef),
    /// The object a special action with a cost is taken for (CR 116.2), and the ability-shaped entry that is the action:
    /// plotting a card (CR 116.2k, 702.170a: "plotting cards from your hand costs {2} less"), suspending, foretelling,
    /// turning a face-down permanent face up, unlocking a door. A special action is not an activated ability (CR 116.1), so
    /// what changes the cost of activating abilities does not apply to it; only a change that names it does.
    SpecialAction(ObjectId, AbilityRef),
}

/// What a cost change does to a cost (CR 601.2f, 118.7, 118.8): generic mana it adds, an amount of mana it takes off,
/// and life it adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CostDelta {
    /// Generic mana added: "costs {1} more" (CR 601.2f).
    pub more: u8,
    /// Life added as an additional cost (CR 118.8, 601.2f): "spells your opponents cast that target this creature cost an
    /// additional 3 life to cast" (Terror of the Peaks). Part of the total cost once the targets are announced, paid with
    /// the rest of it (CR 601.2h), and payable only by a player whose life total is at least the whole of the life the total
    /// cost asks (CR 119.4, 118.3): a cast that cannot pay it is taken back.
    pub life: u8,
    /// The amount of mana the cost is reduced by (CR 118.7). Its generic part reduces only the generic part of the cost
    /// (118.7a); each colored or colorless symbol reduces mana of its own type, and whatever of it the cost does not have
    /// reduces generic mana instead (118.7b-d); a Phyrexian symbol reduces mana of its color (118.7f). `x` is not read
    /// (X is 0 in any mana cost not on the stack, CR 107.3). A hybrid symbol is split by the payer as the reduction is
    /// applied (118.7e: `Label::ReduceAs`), after any hybrid symbol of the cost it could meet is announced (601.2b:
    /// `Label::PayHybridAs`).
    pub less: ManaCost,
    /// The least mana the total cost comes to (CR 601.2f: "once the total cost is determined, any effects that directly
    /// affect the total cost are applied"): "each spell that would cost less than three mana to cast costs three mana to
    /// cast" (Trinisphere). Applied after every increase and every reduction, by generic mana making up the difference, the
    /// greatest of them where several apply; 0 for none. It changes no mana cost, so no mana value (CR 202.3).
    pub at_least: u8,
    /// Generic mana taken off "but never to less than one mana" (CR 601.2f, 118.7): "activated abilities of artifacts you
    /// control cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana" (Forensic
    /// Gadgeteer, Zirda, the Dawnwaker). Applied to the cost every other increase and reduction has left, together, and
    /// leaving at least one mana where there was any; 0 for none.
    pub less_to_one: u8,
}

impl CostDelta {
    /// No change.
    pub const NONE: CostDelta = CostDelta { more: 0, less: ManaCost::ZERO, life: 0, at_least: 0, less_to_one: 0 };

    /// "Costs {n} more."
    pub const fn more(n: u8) -> CostDelta {
        CostDelta { more: n, ..CostDelta::NONE }
    }

    /// "Costs an additional n life" (CR 118.8, 601.2f).
    pub const fn life(n: u8) -> CostDelta {
        CostDelta { life: n, ..CostDelta::NONE }
    }

    /// "Costs {n} less": `n` generic mana off (CR 118.7a).
    pub const fn less(n: u8) -> CostDelta {
        CostDelta { less: ManaCost { generic: n, ..ManaCost::ZERO }, ..CostDelta::NONE }
    }

    /// "Costs {W}{W} less", or "reduced by its mana cost": an amount of mana that may be colored (CR 118.7b-d, f).
    pub const fn less_mana(mana: ManaCost) -> CostDelta {
        CostDelta { less: mana, ..CostDelta::NONE }
    }

    /// "Costs {n} less ... can't reduce the mana in that cost to less than one mana" (`less_to_one`).
    pub const fn less_to_one(n: u8) -> CostDelta {
        CostDelta { less_to_one: n, ..CostDelta::NONE }
    }

    /// "Costs [n] mana to cast" where it would cost less: the total cost's least mana (CR 601.2f, `at_least`).
    pub const fn at_least(n: u8) -> CostDelta {
        CostDelta { at_least: n, ..CostDelta::NONE }
    }
}

/// A static ability that changes what something costs (CR 601.2f, 118.7): what it does to the cost of `costed` when
/// `player` pays, by the permanent `source` that has it (`CostDelta::NONE` when it does not apply). Every increase is
/// applied first, then every reduction, and no part of the cost goes below zero (CR 601.2f); then what sets the least the
/// total may come to (`CostDelta::at_least`).
///
/// `targets` are the targets announced for it, in slot order: the total cost is determined after the targets are
/// chosen (CR 601.2c, 601.2f, 602.2b), so "equip abilities you activate that target this creature cost {2} less" and
/// "costs {1} less for each color of the creature it targets" read them. Once determined the cost is locked in, so what
/// happens to a target afterwards changes nothing. When a cost is judged before anything is announced (whether an action
/// is worth offering), they are the candidate target the action would be announced with, or empty when its targets are
/// chosen later; a change that depends on them then does not apply.
pub type CostChange = fn(&dyn Query, ObjectId, PlayerId, Costed, &[Target]) -> CostDelta;

/// A cost change a resolving spell or ability makes for a duration (CR 611.2a/c, 601.2f): "artifact spells you cast this
/// turn cost {1} less to cast" (Armor Wars), "artifact, instant, and sorcery spells you cast this turn cost {2} less"
/// (Urza, Planeswalker), "spells they cast this turn cost {2} more" (Academy Loremaster). The effect
/// (`EffectKind::SpellCostChange`) names the player whose spells it changes, and lasts whether or not its source is still on
/// the battlefield; `cantrip_mechanics::costs::spells_this_turn` makes one. A card lists these in its `lasting_cost_changes`.
#[derive(Clone, Copy)]
pub struct LastingCostChange {
    /// What it does, in its own words about the player it changes the spells of ("Artifact spells you cast this turn cost
    /// {1} less to cast."): shown beside that player's life total while it lasts.
    pub text: &'static str,
    /// What it does to the total cost of `costed`, a spell of the effect's player (the core asks it of no one else's, and of
    /// no ability): a `CostChange` whose `source` is the effect's source, which may have left the battlefield, so it is
    /// never asked who controls it.
    pub change: CostChange,
}

/// A static ability that gives spells a way of paying their total cost besides mana (CR 601.2f-h): "noncreature spells you
/// cast have improvise" (Ironheart, Clever Champion; Inspiring Statuary's nonartifact spells), "artifact spells you cast have
/// convoke" (Chief Engineer; Dazzling Theater, Eirdu and Caetus's creature or noncreature spells), "spells you cast have
/// delve" (Teval, Arbiter of Virtue). What it gives the spell `spell` that `caster` is casting, by the permanent `source`
/// that has it (`PayWith::NONE` when it gives nothing). The spell has the keyword while it is cast (CR 702.51a, 702.66a,
/// 702.126a), read with its characteristics as a spell; judged before it is cast, `spell` is the card about to be, with
/// those of the face it would be cast as.
pub type PayWithGrant = fn(&dyn Query, ObjectId, PlayerId, ObjectId) -> PayWith;

/// A static ability that lets a player pay a different mana cost rather than an activated ability's (CR 118.9, 602.2b),
/// by the permanent `source` that has it: the cost they may pay instead for `activator` activating the ability `def` of
/// `permanent`, or none. "You may pay {0} rather than pay the equip cost of the first equip ability you activate during
/// each of your turns": Forge Anew. Each one that applies is an activation of its own (`ActionOption::paid_by`).
pub type AbilityAlternative = fn(&dyn Query, &Asker, PlayerId, ObjectId, &ActivatedDef) -> Option<ManaCost>;

/// A mana ability's effect: it resolves at once, never on the stack (CR 605.3). The color is the one chosen for an
/// ability that adds mana of any one color.
pub type ManaEffect = fn(&mut dyn Game, &Asker, Option<Color>) -> Result<(), Suspend>;


/// Whether an ability may be activated or a way of casting taken at all, beyond its cost and timing.
pub type Condition = fn(&dyn Query, &Asker) -> bool;

/// Whether a permanent may be sacrificed to pay a cost.
pub type ObjectFilter = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Who is doing something with what: the source and the player controlling it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asker {
    pub source: ObjectId,
    pub controller: PlayerId,
}

/// One target slot (CR 115.1).
#[derive(Clone, Copy)]
pub struct TargetSpec {
    pub min: u32,
    pub max: u32,
    pub filter: TargetFilter,
    /// What the slot is for, said to the player choosing it: "Spell or permanent to return to its owner's hand". Empty when the card's one slot says
    /// it all. "{player}" stands for the opponent whose slot it is (`EACH_OPPONENT`).
    pub label: &'static str,
    /// "If this spell was kicked, instead choose ..." (CR 702.33a, 601.2b-c): the slot's smallest and largest number of
    /// targets when the spell or the ability's source was kicked, in place of `min` and `max`. The kicker is announced
    /// before the targets are chosen, so the slot is asked once it is known (`TargetSpec::when_kicked`).
    pub if_kicked: Option<(u32, u32)>,
    /// "Any number of target creature cards with total mana value X or less" (CR 601.2c, 603.3d, 202.3): the targets
    /// chosen for the slot together have mana values adding up to at most this, judged as they are chosen (and as they
    /// are chosen anew, CR 115.7). `COUNTED` is the slot's count: the X announced, or what a reflexive ability was given
    /// (`Game::reflexive_trigger_counted`). A candidate worth more than the limit
    /// by itself is no legal target. Once
    /// chosen, each target is checked on its own as the spell or ability resolves (CR 608.2b): one gone leaves the
    /// others. `None` limits nothing.
    pub total_mana_value: Option<u32>,
}

impl TargetSpec {
    /// The most the slot's targets' mana values may add up to (`total_mana_value`), with `counted` read for `COUNTED`.
    pub fn total_limit(&self, counted: u32) -> Option<u32> {
        self.total_mana_value.map(|v| if v == TargetSpec::COUNTED { counted } else { v })
    }

    /// The same slot with `label` as what it is for.
    pub const fn labelled(self, label: &'static str) -> TargetSpec {
        TargetSpec { label, ..self }
    }

    /// A bound of `COUNTED` is the X announced for the spell or ability, or else the number of times it was kicked:
    /// "X target lands" is `min` and `max` both `COUNTED`, "up to X targets" is `min` 0 and `max` `COUNTED`. For a
    /// reflexive triggered ability given a count (`Game::reflexive_trigger_counted`), it is that count: "up to that
    /// many target creatures" (CR 603.12).
    pub const COUNTED: u32 = u32::MAX;

    /// "Any number of targets" (CR 115.1d): as many as there are legal ones to choose - no stack item holds fewer.
    pub const ANY_NUMBER: u32 = u32::MAX - 3;

    /// A `max` of this is "for each opponent, up to one target ...": the slot stands for one slot of its own for each
    /// opponent of the controller (CR 601.2c, 115.1), at most one target each, in seat order. It is not valid inside a mode (a modal slot reads a `max` above 1 as a range). The slot for an opponent
    /// is asked about `Query::slot_opponent`, and an opponent who has left the game keeps their slot, with nothing to
    /// choose, so the others keep their places (CR 800.4a). Its `min` is 0.
    pub const EACH_OPPONENT: u32 = u32::MAX - 2;

    /// Whether a legal target must exist to announce at all (CR 601.2c): a counted slot may be of zero.
    pub const fn required(&self) -> bool {
        self.min > 0 && self.min != TargetSpec::COUNTED
    }

    /// The slot as it is chosen for a spell or source kicked `kicked` times: with the bounds `if_kicked` gives when it
    /// was kicked (CR 702.33a, 601.2c), otherwise as printed.
    pub const fn when_kicked(self, kicked: u8) -> TargetSpec {
        match self.if_kicked {
            Some((min, max)) if kicked > 0 => TargetSpec { min, max, if_kicked: None, ..self },
            _ => self,
        }
    }

    /// Whether the slot is "any number of targets" (CR 115.1d).
    pub const fn is_any_number(&self) -> bool {
        self.min == 0 && self.max == TargetSpec::ANY_NUMBER
    }

    /// The slot's smallest and largest number of targets, with `counted` read for a bound that is `COUNTED`.
    pub fn bounds(&self, counted: u32) -> (u32, u32) {
        let of = |v: u32| if v == TargetSpec::COUNTED { counted } else { v };
        (of(self.min), of(self.max))
    }
}

/// A cost (CR 118).
#[derive(Clone, Copy)]
pub struct Cost {
    pub mana: ManaCost,
    /// {T} (CR 107.5).
    pub tap: bool,
    pub sacrifice_self: bool,
    /// "Sacrifice [the card that gives this ability]" in an ability another permanent gives (CR 201.5a: the name means that
    /// object, not any of that name): Deconstruction Hammer's "{3}, {T}, Sacrifice Deconstruction Hammer", which the Hammer
    /// gives the equipped creature. Paid by sacrificing the permanent the activation names as what gave the ability
    /// (`ActionOption::granted_by`), which the payer must control (CR 701.21a); an ability no permanent gave can't pay it.
    pub sacrifice_granter: bool,
    /// Discard this card (cycling, CR 702.29a).
    pub discard_self: bool,
    /// Choose this many permanents you control that pass the filter, and sacrifice them, return them to their owners'
    /// hands or exile them (CR 601.2h, 118.3): chosen as the cost is chosen, moved as it is paid.
    pub permanents: Option<PermanentCost>,
    /// A loyalty cost (CR 606.4): positive adds, negative removes.
    pub loyalty: Option<i8>,
    /// Pay this much life (CR 119.4): payable only with at least that much, and not at all while the payer can't lose life
    /// (CR 119.8).
    pub life: u16,
    /// Pay energy counters (CR 107.14, 118.3): Primal Prayers, Amped Raptor.
    pub energy: u8,
    /// Remove this many counters of this kind from the source (CR 602.1, 118.3).
    pub remove_counters: Option<(CounterKind, u8)>,
    /// Put this many counters of this kind on the source (CR 118.3, 602.2b): "{T}, Put a page counter on this artifact"
    /// (Mazemind Tome). Payable only while nothing says counters of that kind can't be put on it (CR 101.2, 118.3:
    /// Solemnity's ruling, `CardDef::counter_bans`); they are put by the activating player, so replacement effects on
    /// counters being put apply (CR 614.1a), but not those of counters "an effect" puts (`CounterPlacement::effect`).
    pub put_counters: Option<(CounterKind, u8)>,
    /// Remove a counter of any kind from the source (CR 118.3): the payer names the kind when it has more than one.
    pub remove_any_counter: bool,
    /// Remove this many counters, of any kinds, from among the permanents the payer controls that pass the filter (CR 118.3,
    /// 122.1): "remove three counters from among creatures you control" (the casts Dawnhand Dissident permits; Tayam,
    /// Luminous Enigma's ability). Which counters, from which permanents, is chosen as the cost is chosen (CR 601.2h), and
    /// they are removed as it is paid. Payable only while there are that many counters among them.
    pub remove_counters_among: Option<(u8, ObjectFilter)>,
    /// Exile this card (from the zone the ability functions in: eternalize, CR 702.129a).
    pub exile_self: bool,
    /// Discard this many cards from your hand that pass the filter (CR 701.9), chosen as the cost is chosen.
    pub discard: Option<(u8, ObjectFilter)>,
    /// Pay X life, X announced with the spell (CR 118.8, 601.2b): Toxic Deluge, Vicious Rivalry.
    pub pay_x_life: bool,
    /// Exile this many cards from your graveyard, other than the source (CR 702.138a escape, and "exile three cards
    /// from your graveyard"), chosen as the cost is chosen (CR 601.2h).
    pub exile_from_graveyard: u8,
    /// The cards `exile_from_graveyard` exiles must pass this filter: "Exile three creature cards from your graveyard"
    /// (Drivnod, Carnage Dominus). `None` takes any card.
    pub exile_from_graveyard_of: Option<ObjectFilter>,
    /// Exert this permanent (CR 701.43a): it won't untap during its controller's next untap step.
    pub exert: bool,
    /// Tap any number of untapped creatures you control, other than the source, with total power N or more
    /// (CR 702.122a crew, CR 702.171a saddle).
    pub tap_creatures: Option<TapCreatures>,
    /// Emerge (CR 702.119a): the mana in this cost comes down by the mana value of the permanents it sacrifices
    /// (`permanents`), a cost reduction that takes generic mana only (CR 118.7).
    pub emerge: bool,
    /// Exile this many cards from your hand that pass the filter, other than the source (CR 118.9: "exile a red card from
    /// your hand"), chosen as the cost is chosen.
    pub exile_from_hand: Option<(u8, ObjectFilter)>,
    /// The `permanents` are tapped instead of sacrificed or returned (CR 118.3, 701.26): "Tap an untapped creature you
    /// control", chosen as the cost is chosen and tapped as it is paid. It is not the {T} symbol (CR 107.5), so a
    /// summoning-sick creature, or the source itself, may be tapped for it.
    pub tap_permanents: bool,

    /// Unattach this Equipment (CR 701.3d, 118.3): payable only while it is attached to something, since a cost that
    /// changes nothing cannot be paid.
    pub unattach_self: bool,
    /// Return this permanent to its owner's hand (CR 118.3): payable only while it is on the battlefield.
    pub return_self: bool,
    /// A part of the cost whose size the payer decides (CR 601.2b, 601.2h).
    pub variable: Option<VariableCost>,
    /// Discard your hand (CR 701.9): every card in it, with no choice to make and payable with an empty hand.
    pub discard_hand: bool,
    /// Collect evidence N (CR 701.59a): exile cards with total mana value N or more from your graveyard, other than the
    /// source, chosen as the cost is chosen.
    pub collect_evidence: u8,
    /// Behold N (CR 701.4a): for each, choose a permanent you control or reveal a card from your hand that passes the
    /// filter ("behold a Dragon"), chosen as the cost is chosen. Nothing moves: a permanent is only chosen, a hand card
    /// is revealed as the cost is paid (CR 601.2h). Payable only with that many to choose among. Dispelling Exhale,
    /// Caustic Exhale, Kinsbaile Aspirant. Paid when a spell is cast only: an activation does not choose or pay it yet.
    /// The quality's name ("Dragon") is what the prompt says.
    pub behold: Option<(u8, ObjectFilter, &'static str)>,
    /// Blight N (CR 701.68a): put this many -1/-1 counters on a creature you control, chosen as the cost is chosen and
    /// put as it is paid. Payable only with a creature to put them on (CR 701.68b). A spell's (Requiting Hex, Cinder
    /// Strike) or an activated ability's (Dawnhand Dissident, Gristle Glutton, Sting-Slinger).
    pub blight: u8,
    /// Harmonize's additional part (CR 702.180a-b): tap up to one untapped creature you control, and the total cost comes down
    /// by generic mana equal to its power (never by colored mana, CR 601.2f). The creature is chosen as the cost is chosen
    /// (CR 601.2b), its power read as the total cost is determined (CR 601.2f), and it is tapped as the cost is paid. It is not
    /// a {T} ability's cost (CR 107.5), so a summoning-sick creature may be tapped. Only a spell's way of casting carries it.
    pub tap_to_reduce: bool,
    /// Craft's materials (CR 702.167a): "Exile [materials] from among permanents you control and/or cards in your
    /// graveyard", chosen as the cost is chosen (CR 601.2h by way of 602.2b) and exiled as it is paid, together with
    /// the source when the cost also exiles it (`exile_self`). Only an activated ability's cost carries it.
    pub materials: Option<&'static Materials>,
}

/// What a craft ability exiles besides its source (CR 702.167a): `count` objects, or `count` or more, from among the
/// permanents the payer controls and the cards in their graveyard, the source apart, each passing `filter`. The filter
/// reads where the object is: a material named by a type alone ("artifact") is a permanent of that type or a card of
/// that type in the graveyard (CR 702.167b), one named as a card ("red instant and/or sorcery cards") is a graveyard
/// card only.
#[derive(Clone, Copy)]
pub struct Materials {
    pub count: u8,
    /// "One or more", "four or more": at least `count`, as many more as the payer likes.
    pub or_more: bool,
    pub filter: ObjectFilter,
    /// What one material is, as the prompt says it ("artifact", "creature", "Cave").
    pub quality: &'static str,
    /// Materials named one by one ("a Dinosaur, a Merfolk, a Pirate, and a Vampire"): each a separate object passing its
    /// own filter (the Throne of the Grim Captain ruling: one changeling is not enough), chosen one after another. Empty
    /// for materials of one kind; when it is not, `count` is its length and `filter` takes what any of them takes.
    pub parts: &'static [MaterialPart],
}

/// One of materials named one by one: what it must be, and what the prompt calls it.
#[derive(Clone, Copy)]
pub struct MaterialPart {
    pub filter: ObjectFilter,
    pub quality: &'static str,
}

/// A cost part whose size is the payer's to set, chosen as the cost is chosen.
#[derive(Clone, Copy)]
pub enum VariableCost {
    /// Sacrifice X permanents you control that pass the filter, X announced with the spell though its mana cost has no
    /// {X} (CR 601.2b, 107.3a): Immoral Bargain.
    SacrificeX(ObjectFilter),
    /// Sacrifice one or more permanents you control that pass the filter (CR 701.21a): as an optional additional cost, the
    /// spell's `paid` is how many were. Plumb the Forbidden.
    SacrificeOneOrMore(ObjectFilter),
    /// "As an additional cost to cast this spell, you may sacrifice any number of [permanents]. This spell costs {N} less to
    /// cast for each permanent sacrificed this way" (Rottenmouth Viper): how many is announced as the spell is cast (CR
    /// 601.2b), so the total cost is determined knowing it (CR 601.2f) - `less_each` generic mana off for each, a reduction
    /// applied after any increase, never below {0} (CR 118.7a) - and which ones are chosen as the cost is paid, after the
    /// mana abilities (CR 601.2g-h). The number is no X (its mana cost has none, CR 107.3): it is kept as the spell's
    /// `paid`, how many times the additional cost was paid, and the spell's `x` stays 0.
    SacrificeAnyNumber { filter: ObjectFilter, less_each: u8 },
    /// Exile any number of other cards from your graveyard with this many card types or more among them (CR 205.2a):
    /// Nethergoyf's escape.
    ExileFromGraveyardWithTypes(u8),
}

/// The cost of crew (CR 702.122a) and saddle (CR 702.171a): tap other untapped creatures with total power `power` or
/// more. Summoning sickness does not matter, since it is not their own {T} (CR 302.6).
#[derive(Clone, Copy)]
pub struct TapCreatures {
    pub power: u8,
    /// The keyword with which a creature tapped for this cost counts as though its power were 2 greater: crew's
    /// `CrewsAsThoughPowerTwoGreater` (CR 702.122b), saddle's `SaddlesAsThoughPowerTwoGreater` (CR 702.171c). None for a
    /// cost that is neither (teamwork).
    pub greater_with: Option<Keyword>,
}

/// What a permanents cost does with the permanents it chose (`PermanentCost::action`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermanentAction {
    /// "Sacrifice N [permanents]" (CR 701.21). Also what a cost that taps them carries (`Cost::tap_permanents`, which
    /// moves nothing).
    Sacrifice,
    /// "Return N [permanents] you control to their owner's hand".
    ReturnToHand,
    /// "Exile N [permanents] you control" (CR 118.3): The Soul Stone's and Food Chain's "Exile a creature you control",
    /// Soul Exchange's additional cost. An activated ability's exiles are linked to its source (CR 607.2a: cards "exiled
    /// with" it); an ability's are its cost objects for "the exiled creature" (`Query::cost_objects`, CR 608.2h).
    Exile,
}

#[derive(Clone, Copy)]
pub struct PermanentCost {
    pub count: u8,
    pub filter: ObjectFilter,
    pub action: PermanentAction,
    /// Only tapped permanents pay ("return a tapped creature you control", web-slinging, CR 702.188a). Whether one is
    /// tapped is judged as the cost is paid (CR 601.2h), after the mana abilities of CR 601.2g: the permanents are
    /// chosen once the mana is paid, so one tapped for mana while paying may be among them (Spider-Sense's ruling,
    /// 2025-09-19).
    pub tapped: bool,
}

impl Cost {
    /// Whether the cost is {T} and nothing else (CR 107.5): what a basic land's mana ability costs. Every part is named, so
    /// a part added to `Cost` must be judged here too.
    pub fn is_tap_only(&self) -> bool {
        let Cost {
            mana,
            tap,
            sacrifice_self,
            sacrifice_granter,
            discard_self,
            permanents,
            loyalty,
            life,
            energy,
            remove_counters,
            put_counters,
            remove_any_counter,
            remove_counters_among,
            exile_self,
            discard,
            pay_x_life,
            exile_from_graveyard,
            exile_from_graveyard_of,
            exert,
            tap_creatures,
            emerge,
            exile_from_hand,
            tap_permanents,
            unattach_self,
            return_self,
            variable,
            discard_hand,
            collect_evidence,
            behold,
            blight,
            tap_to_reduce,
            materials,
        } = *self;
        tap && mana.is_zero() && mana.x == 0
            && !sacrifice_self
            && !sacrifice_granter
            && !discard_self
            && permanents.is_none()
            && loyalty.is_none()
            && life == 0
            && energy == 0
            && remove_counters.is_none()
            && put_counters.is_none()
            && !remove_any_counter
            && remove_counters_among.is_none()
            && !exile_self
            && discard.is_none()
            && !pay_x_life
            && exile_from_graveyard == 0
            && exile_from_graveyard_of.is_none()
            && !exert
            && tap_creatures.is_none()
            && !emerge
            && exile_from_hand.is_none()
            && !tap_permanents
            && !unattach_self
            && !return_self
            && variable.is_none()
            && !discard_hand
            && collect_evidence == 0
            && behold.is_none()
            && blight == 0
            && !tap_to_reduce
            && materials.is_none()
    }

    /// This cost paid `n` times over (escalate, CR 702.120a, once for each mode beyond the first): its mana, life,
    /// discards, permanents and graveyard exiles each multiplied. The parts that can only be paid once are left as
    /// they are.
    pub fn times(self, n: u8) -> Cost {
        let mut c = self;
        for _ in 1..n {
            c.mana.generic += self.mana.generic;
            for i in 0..6 {
                c.mana.symbols[i] += self.mana.symbols[i];
            }
        }
        if n == 0 {
            c.mana = ManaCost::ZERO;
        }
        c.life = self.life.saturating_mul(u16::from(n));
        c.energy = self.energy.saturating_mul(n);
        c.exile_from_graveyard = self.exile_from_graveyard.saturating_mul(n);
        c.blight = self.blight.saturating_mul(n);
        c.remove_counters_among = self.remove_counters_among.map(|(k, f)| (k.saturating_mul(n), f)).filter(|_| n > 0);
        c.exile_from_hand = self.exile_from_hand.map(|(k, f)| (k.saturating_mul(n), f)).filter(|_| n > 0);
        c.discard = self.discard.map(|(k, f)| (k.saturating_mul(n), f)).filter(|_| n > 0);
        c.permanents = self.permanents.map(|p| PermanentCost { count: p.count.saturating_mul(n), ..p }).filter(|_| n > 0);
        c
    }

    /// This cost and another paid together: the parts of the same kind add (an additional cost's discard and an
    /// escalate cost's discard are discards of one choice). A card writes "{1}, {T}" as `cost("{1}").and(TAP)`.
    pub const fn and(self, other: Cost) -> Cost {
        let mut c = self;
        c.mana.generic += other.mana.generic;
        let mut i = 0;
        while i < 6 {
            c.mana.symbols[i] += other.mana.symbols[i];
            i += 1;
        }
        let mut i = 0;
        while i < 6 {
            if c.mana.hybrid[i].is_none() {
                c.mana.hybrid[i] = other.mana.hybrid[i];
            }
            i += 1;
        }
        let mut i = 0;
        while i < 5 {
            c.mana.phyrexian[i] += other.mana.phyrexian[i];
            c.mana.twobrid[i] += other.mana.twobrid[i];
            i += 1;
        }
        c.mana.x += other.mana.x;
        c.tap |= other.tap;
        c.sacrifice_self |= other.sacrifice_self;
        c.sacrifice_granter |= other.sacrifice_granter;
        c.discard_self |= other.discard_self;
        c.exile_self |= other.exile_self;
        c.pay_x_life |= other.pay_x_life;
        c.emerge |= other.emerge;
        c.tap_to_reduce |= other.tap_to_reduce;
        c.tap_permanents |= other.tap_permanents;
        c.remove_any_counter |= other.remove_any_counter;
        if c.loyalty.is_none() {
            c.loyalty = other.loyalty;
        }
        if c.remove_counters.is_none() {
            c.remove_counters = other.remove_counters;
        }
        // Two removals from among permanents are one choice, of the first's permanents: no card has two with different filters.
        c.remove_counters_among = match (self.remove_counters_among, other.remove_counters_among) {
            (Some((a, f)), Some((b, _))) => Some((a.saturating_add(b), f)),
            (None, b) => b,
            (a, None) => a,
        };
        if c.put_counters.is_none() {
            c.put_counters = other.put_counters;
        }
        c.life += other.life;
        c.energy = c.energy.saturating_add(other.energy);
        c.exile_from_graveyard += other.exile_from_graveyard;
        if c.exile_from_graveyard_of.is_none() {
            c.exile_from_graveyard_of = other.exile_from_graveyard_of;
        }
        c.exile_from_hand = match (self.exile_from_hand, other.exile_from_hand) {
            (Some((a, f)), Some((b, _))) => Some((a + b, f)),
            (None, b) => b,
            (a, None) => a,
        };
        c.discard = match (self.discard, other.discard) {
            (Some((a, f)), Some((b, _))) => Some((a + b, f)),
            (None, b) => b,
            (a, None) => a,
        };
        c.permanents = match (self.permanents, other.permanents) {
            (Some(a), Some(b)) => {
                // A tapped permanent chosen after the mana and one chosen before are not one choice.
                assert!(a.tapped == b.tapped, "two permanent costs, only one of them for tapped permanents");
                // Nor are permanents sacrificed and permanents exiled.
                assert!(a.action as u8 == b.action as u8, "two permanent costs that do different things with them");
                Some(PermanentCost { count: a.count + b.count, ..a })
            }
            (None, b) => b,
            (a, None) => a,
        };
        if c.tap_creatures.is_none() {
            c.tap_creatures = other.tap_creatures;
        }
        c.exert |= other.exert;
        c.unattach_self |= other.unattach_self;
        c.return_self |= other.return_self;
        if c.variable.is_none() {
            c.variable = other.variable;
        }
        c.discard_hand |= other.discard_hand;
        c.collect_evidence += other.collect_evidence;
        // Two beholds add only when they ask for the same quality (CR 701.4a): one filter can't stand for two.
        c.behold = match (self.behold, other.behold) {
            (Some((a, f, name)), Some((b, _, other_name))) => {
                assert!(same_str(name, other_name), "two beholds of different qualities can't be combined");
                Some((a + b, f, name))
            }
            (None, b) => b,
            (a, None) => a,
        };
        c.blight += other.blight;
        if c.materials.is_none() {
            c.materials = other.materials;
        }
        c
    }

    pub const NONE: Cost =
        Cost {
            mana: ManaCost::ZERO,
            tap: false,
            sacrifice_self: false,
            sacrifice_granter: false,
            discard_self: false,
            permanents: None,
            loyalty: None,
            life: 0,
            energy: 0,
            remove_counters: None,
            put_counters: None,
            remove_any_counter: false,
            remove_counters_among: None,
            exile_self: false,
            discard: None,
            pay_x_life: false,
            exile_from_graveyard: 0,
            exile_from_graveyard_of: None,
            exert: false,
            tap_creatures: None,
            emerge: false,
            exile_from_hand: None,
            tap_permanents: false,

            unattach_self: false,
            return_self: false,
            variable: None,
            discard_hand: false,
            collect_evidence: 0,
            behold: None,
            blight: 0,
            tap_to_reduce: false,
            materials: None,
        };

    pub const fn sacrifice(count: u8, filter: ObjectFilter) -> Cost {
        Cost { permanents: Some(PermanentCost { count, filter, action: PermanentAction::Sacrifice, tapped: false }), ..Cost::NONE }
    }

    /// Tap `count` untapped permanents you control that pass the filter, as a cost (CR 118.3).
    pub const fn tap_untapped(count: u8, filter: ObjectFilter) -> Cost {
        Cost { permanents: Some(PermanentCost { count, filter, action: PermanentAction::Sacrifice, tapped: false }), tap_permanents: true, ..Cost::NONE }
    }

    pub const fn return_to_hand(count: u8, filter: ObjectFilter) -> Cost {
        Cost { permanents: Some(PermanentCost { count, filter, action: PermanentAction::ReturnToHand, tapped: false }), ..Cost::NONE }
    }

    /// Return `count` tapped permanents you control that pass the filter to their owners' hands (web-slinging, CR
    /// 702.188a): chosen once the mana is paid (`PermanentCost::tapped`).
    pub const fn return_tapped_to_hand(count: u8, filter: ObjectFilter) -> Cost {
        Cost { permanents: Some(PermanentCost { count, filter, action: PermanentAction::ReturnToHand, tapped: true }), ..Cost::NONE }
    }

    /// Exile `count` permanents you control that pass the filter, as a cost (CR 118.3): "Exile a creature you control".
    pub const fn exile_permanents(count: u8, filter: ObjectFilter) -> Cost {
        Cost { permanents: Some(PermanentCost { count, filter, action: PermanentAction::Exile, tapped: false }), ..Cost::NONE }
    }

    pub const fn mana(mana: ManaCost) -> Cost {
        Cost { mana, ..Cost::NONE }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timing {
    Instant,
    /// Main phase, own turn, empty stack (CR 307.1).
    Sorcery,
    /// Any time its controller has priority during their own turn: foretell (CR 702.143a).
    OwnTurn,
    /// Whenever its card could be cast from the hand: at instant speed for an instant or a card with flash, at sorcery
    /// speed for any other (CR 702.62a suspend).
    AsCast,
}

/// Suspend (CR 702.62a): the card may be exiled from the hand with `count` time counters for `cost`, and is cast for
/// free when the last is removed at the beginning of its owner's upkeeps.
#[derive(Clone, Copy)]
pub struct SuspendDef {
    pub count: u8,
    pub cost: ManaCost,
}

/// Whether two texts are the same, as the compiler can ask it.
pub(crate) const fn same_str(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// An activated ability (CR 602). A card writes one with `ability` or `mana_ability` (`crate::ability`).
#[derive(Clone, Copy)]
pub struct ActivatedDef {
    /// The ability's text, which a scenario's `Ability:` names it by.
    pub text: &'static str,
    pub cost: Cost,
    pub timing: Timing,
    pub targets: &'static [TargetSpec],
    pub effect: Effect,
    /// A mana ability's own effect (CR 605.1a), where it does more than add the mana `adds` and `adds_any` list: it
    /// resolves immediately, and `effect` is unused. One with {X} in its cost has X announced first (CR 107.3a, 605.3a);
    /// the effect reads it as `Obj::x` of its source. A mana ability with none adds what it lists
    /// (`is_mana_ability`, `add_listed`).
    pub mana: Option<ManaEffect>,
    /// A mana ability that adds mana of any one color: the activation names the color (CR 106.5).
    pub any_color: bool,
    /// The colors an `any_color` ability may name ("Add {B}, {R}, or {G}"); none means all five.
    pub colors: ColorSet,
    /// The mana types an `any_color` ability may name are those the permanent sacrificed to activate it could produce
    /// (CR 106.7): "add one mana of any type that land could produce". Named by colorless as `Action::mana` `Some(C)`; an
    /// ability that adds nothing when the permanent could produce nothing (CR 106.5).
    pub colors_from_sacrifice: bool,
    /// The colors an `any_color` ability may name, worked out from the game when it is offered ("any of the exiled
    /// card's colors": a linked ability, CR 607.2a). With no colors there is none to choose: the ability is still
    /// offered, and adds no mana (CR 106.5). Overrides `colors`.
    pub colors_from: Option<fn(&dyn Query, &Asker) -> ColorSet>,
    /// The zone it functions from: the battlefield, or the hand for cycling and the like (CR 113.6).
    pub from: ZoneKind,
    /// What a mana ability adds, for whoever wants to know before activating it: fixed kinds, and how many of one
    /// chosen color.
    pub adds: ManaPool,
    pub adds_any: u8,
    pub condition: Option<Condition>,
    /// "Activate only once each turn" (CR 602.5b-c): counted for this object and for each instance of the ability on
    /// its own, whoever activated it.
    pub once_each_turn: bool,
    /// Exhaust (CR 702.177a): "Activate each exhaust ability only once" by this permanent. Implies `once`.
    pub exhaust: bool,
    /// "Activate only once" (CR 602.5b; exhaust 702.177a, power-up 702.193a): this object has never activated this
    /// ability. Asked of the event history, where every activation of an ability, a mana ability's included, is recorded;
    /// a permanent that left and came back is a new object (CR 400.7) with the ability unused.
    pub once: bool,
    /// Its own cost change (CR 601.2f by way of 602.2b): "this ability costs {1} less to activate for each ...", power-up's
    /// "this ability's cost is reduced by this permanent's mana cost" (CR 702.193a). Its `source` is the object whose
    /// ability it is, and it is asked only for this ability.
    pub cost_change: Option<CostChange>,
    /// Its modes, chosen on activation before its costs (CR 602.2b, 700.2); `effect` and `targets` are then unused.
    pub modal: Option<&'static ModalDef>,
    /// For a mana ability: its mana can only be spent as this restriction allows (CR 106.6), and goes to the pool
    /// apart from unrestricted mana.
    pub restricted: Option<ManaRestriction>,
    /// Its mana cost is a waterbend cost (CR 701.67a): each generic mana of it may be paid by tapping an untapped
    /// artifact or creature its controller has, instead.
    pub waterbend: bool,
    /// "For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that
    /// mana" (CR 118.3, 601.2h): each generic mana of it may be paid by tapping an untapped creature (Heirloom Epic).
    pub tap_creatures_for_mana: bool,
    /// A special action (CR 116.2): taken with priority, paid for and done at once, using no stack and with no chance to
    /// respond. Plot (CR 702.170a) is one. It is not an activation (CR 602): nothing triggers on it.
    pub special_action: bool,
    /// An equip ability (CR 702.6a): "equip abilities" are named as a class by cards that change how they work.
    pub equip: bool,
    /// Its targets are chosen after X is announced (CR 601.2b-c by way of 602.2b), because what may be targeted depends on
    /// X ("target creature card in your graveyard with mana value X": Likeness Looter): never with the action, even for
    /// one slot of one target.
    pub late_targets: bool,
    /// Plot's special action (CR 702.170a, 116.2k): "plotting cards" is named as a class by a card that changes its cost
    /// (Doc Aurlock, Grizzled Genius).
    pub plot: bool,
}

fn no_effect(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

pub(crate) fn no_match(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    None
}

impl ActivatedDef {
    /// A loyalty ability (CR 606.2): an activated ability with a loyalty symbol in its cost.
    pub const fn is_loyalty_ability(&self) -> bool {
        self.cost.loyalty.is_some()
    }

    /// A mana ability (CR 605.1a): one with an effect of its own that adds mana, or one that lists the mana it adds.
    pub const fn is_mana_ability(&self) -> bool {
        if self.mana.is_some() || self.adds_any > 0 {
            return true;
        }
        let mut i = 0;
        while i < 6 {
            if self.adds.0[i] > 0 {
                return true;
            }
            i += 1;
        }
        false
    }

    pub const BLANK: ActivatedDef = ActivatedDef {
        text: "",
        cost: Cost::NONE,
        timing: Timing::Instant,
        targets: &[],
        effect: no_effect,
        mana: None,
        any_color: false,
        colors: ColorSet::NONE,
        colors_from: None,
        colors_from_sacrifice: false,
        from: ZoneKind::Battlefield,
        adds: ManaPool([0; 6]),
        adds_any: 0,
        condition: None,
        once_each_turn: false,
        exhaust: false,
        once: false,
        cost_change: None,
        restricted: None,
        waterbend: false,
        tap_creatures_for_mana: false,
        equip: false,
        late_targets: false,
        plot: false,
        special_action: false,
        modal: None,
    };
}

impl TriggeredDef {
    /// Whether `event` is this ability's trigger event (CR 603.2), for the ability `asker` names: its source and the player
    /// who controls the ability.
    #[inline]
    pub fn fires(&self, q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
        if let Some(only_if) = self.only_if
            && !only_if(q, asker)
        {
            return None;
        }
        match self.asked {
            Some(asked) => asked(q, asker, event),
            None => (self.matches)(q, asker.source, event),
        }
    }

    /// Whether it could look back on some event (`CardTable::looks_back`): it listens for one that can. What `CardTable::look_back` and
    /// `settle::Then` count.
    pub fn may_look_back(&self) -> bool {
        self.listens.intersects(EventMask::LOOKS_BACK)
    }

    pub const BLANK: TriggeredDef = TriggeredDef {
        text: "",
        listens: EventMask(0),
        matches: no_match,
        asked: None,
        targets: &[],
        effect: no_effect,
        anywhere: false,
        condition: None,
        only_if: None,
        once_per_batch: false,
        batch_by_data: false,
        mana_ability: false,
        mana_yield: TriggeredMana::Unstated,
        sums_per_subject: false,
        triggers_each_turn: 0,
        chapter: 0,
        state: None,
        modal: None,
        divide: None,
        subject_chooses_targets: false,
        ward: false,
        on_sacrifice: false,
    };
}

/// What a triggered mana ability (CR 605.1b) adds when a permanent is tapped for mana, to the player who tapped it: stated
/// so the advice of what a player can pay counts it before anything is tapped (CR 601.2g), as the effect itself is code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriggeredMana {
    /// Not stated: the advice counts nothing for it.
    Unstated,
    /// One mana of this type: "add an additional {G}" (Nissa, Who Shakes the World; High Tide; Badgermole Cub).
    One(Mana),
    /// One mana of any type that permanent produced (Mana Flare, Lavaleaper).
    OneOfProduced,
    /// One mana of the color its source chose as it entered (Utopia Sprawl).
    ChosenColor,
}

/// What a half of a fuse card has that the core's fused split spell (CR 702.102) does not make - each a choice or cost of
/// its own as a spell is cast (CR 601.2b-f), or text the core runs otherwise - named for the refusal; none for a half the
/// fused spell plays exactly. No printed fuse card has any of these.
pub fn fused_half_unbuilt(half: &CardDef) -> Option<&'static str> {
    [
        (half.spell.is_none(), "no spell"),
        (half.modal.is_some(), "modes"),
        (half.modes_up_to.is_some(), "a number of modes"),
        (half.cost.x > 0, "an {X}"),
        (half.kicker.is_some(), "a kicker"),
        (half.optional_cost.is_some(), "an optional additional cost"),
        (half.additional_cost.is_some(), "an additional cost"),
        (!half.additional_choice.is_empty(), "a choice of additional costs"),
        (half.gift.is_some(), "a gift"),
        (half.divide.is_some(), "a divided total"),
        (half.late_targets, "targets chosen late"),
        (half.cost_of_targets.is_some(), "a cost read from its targets"),
        (half.cast_fact.is_some(), "a fact kept as it is cast"),
        (!half.end_step_riders.is_empty(), "an end step rider"),
        (!half.ways.is_empty(), "ways of its own to be cast"),
        (half.overload.is_some(), "overload"),
        (half.cast_only_if.is_some(), "a condition on casting it"),
        (!half.own_cost_changes.is_empty(), "its own cost changes"),
        (half.pay_with.any(), "convoke, delve or improvise"),
        (half.cast_as_though_flash, "a cast as though it had flash"),
        (half.paradigm.is_some(), "paradigm"),
        (half.rebound, "rebound"),
        (half.splice.is_some(), "splice"),
    ]
    .into_iter()
    .find_map(|(has, what)| has.then_some(what))
}

/// A triggered ability (CR 603).
#[derive(Clone, Copy)]
pub struct TriggeredDef {
    pub text: &'static str,
    pub listens: EventMask,
    pub matches: TriggerMatch,
    /// The trigger event asked with the ability's controller (`AskedMatch`), in place of `matches` when set.
    pub asked: Option<AskedMatch>,
    pub targets: &'static [TargetSpec],
    pub effect: Effect,
    /// Functions outside the battlefield too (the ability checks the zone itself), as "at the beginning of your
    /// upkeep, if this card is in your graveyard" does.
    pub anywhere: bool,
    pub condition: Option<TriggerCondition>,
    /// "This ability triggers only if ..." (CR 702.169c, a Case's "Solved — [triggered ability]"): asked with the trigger
    /// event, as part of it, and never again. Not an intervening "if" (CR 603.4), which `condition` is: that is asked again
    /// as the ability resolves.
    pub only_if: Option<TriggerCondition>,
    /// A "one or more" ability (CR 603.2c): the events of one simultaneous batch that it matches trigger it once.
    /// `matches` is asked of each event and the subject objects of those that match are the batch's members, which
    /// the resolving ability reads with `Query::event_objects`.
    pub once_per_batch: bool,
    /// With `once_per_batch`: events of the batch whose `data[1]` differ are different batches, each triggering it once (CR 603.2c):
    /// "whenever one or more cards are put into a library" is one for each library (Wan Shi Tong, All-Knowing).
    pub batch_by_data: bool,
    /// A triggered mana ability (CR 605.1b): it triggers on the tapping of a permanent for mana (`Event::TappedForMana`) and resolves at
    /// once, not on the stack, as part of the mana ability that triggered it (CR 605.4a). `matches` is asked of every permanent that has
    /// one; `effect` adds the mana.
    pub mana_ability: bool,
    /// What a triggered mana ability adds to the player who tapped the permanent each time it triggers (`TriggeredMana`): what
    /// the advice of whether a cost can be paid counts for each mana source it would trigger on (CR 601.2g, 605.1b).
    pub mana_yield: TriggeredMana,
    /// Events of one batch that are about the same subject trigger it once, with their `data[0]` added up (CR 120.2,
    /// 603.2c): damage dealt to one creature at once is one event, however many sources dealt it. "Whenever a creature
    /// you control is dealt damage ... that much damage" (The Sensational She-Hulk, Screaming Nemesis).
    pub sums_per_subject: bool,
    /// "This ability triggers only N times each turn" (no numbered rule; CR 603.2h is "do this only once each turn"): 0 for no limit. A triggering that would exceed it
    /// does not happen, and one that happens uses one up.
    pub triggers_each_turn: u8,
    /// A Saga's chapter ability (CR 714.2): it triggers when lore counters are put on its source and their number
    /// becomes at least this, having been less. `listens` and `matches` are not used; the core owns the trigger.
    pub chapter: u8,
    /// A state trigger (CR 603.8): it triggers when this game state is true, asked of its source and controller, rather
    /// than on an event - at once, and again only once it has left the stack (resolved, countered or otherwise) and the
    /// state still holds. `listens` and `matches` are not used; the core owns the trigger.
    pub state: Option<TriggerCondition>,
    /// A modal ability's modes (CR 700.2): "choose one". Chosen, with each chosen mode's targets, as the ability is put on
    /// the stack (CR 603.3c); `effect` and `targets` are then unused.
    pub modal: Option<&'static ModalDef>,
    /// Its first target slot divides a total among its targets (CR 601.2d).
    pub divide: Option<Divide>,
    /// "That player chooses target ..." (CR 603.3d): the player the event was about, not the ability's controller, chooses its
    /// targets (Oath of Druids and Oath of Mages at each player's upkeep). Targeting is otherwise the controller's: legality, hexproof
    /// and the ability itself stay theirs.
    pub subject_chooses_targets: bool,
    /// A ward ability (CR 702.21a): a targeting rule can stop it from triggering (`TargetingRuleKind::SuppressWard`).
    pub ward: bool,
    /// Its trigger event is a player sacrificing a permanent (CR 701.21a), not the zone change the sacrifice causes: "whenever
    /// you sacrifice a land" (Baloth Prime, Tireless Tracker). A rule that stops a creature dying from causing abilities to
    /// trigger leaves it alone (Hushbringer's rulings: "whenever you sacrifice a creature" still triggers).
    pub on_sacrifice: bool,
}

#[derive(Clone, Copy)]
pub enum AbilityDef {
    Activated(ActivatedDef),
    Triggered(TriggeredDef),
    /// A static ability an effect gives an object (CR 613.1f, 613.7a): "[this creature] gains 'This creature's base power is
    /// equal to the number of cards in your hand.'" It is one of a card's `delayed` list, granted with `EffectKind::AddAbility`.
    /// It affects only the object that has it, with that object as its source (its "you" is the object's controller), in a
    /// layer after the abilities are settled (`Layer::PtDefine` on): see `StaticDef`.
    Static(StaticDef),
}

impl AbilityDef {
    pub fn text(&self) -> &'static str {
        match self {
            AbilityDef::Activated(a) => a.text,
            AbilityDef::Triggered(t) => t.text,
            AbilityDef::Static(s) => s.text,
        }
    }
}

/// Which keyword a way is, where a card asks how another was cast ("if that creature was cast for its warp cost"): the
/// way's identity, set by the keyword's own constructor. Other keywords' ways are `Other` until a card asks about them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WayKind {
    Other,
    /// Warp (CR 702.185a).
    Warp,
    /// Web-slinging (CR 702.188a).
    WebSlinging,
}

/// An alternative way to cast a card (CR 118.9): flashback (CR 702.34) from the graveyard, Fireblast's sacrifice.
#[derive(Clone, Copy)]
pub struct WayDef {
    pub text: &'static str,
    pub cost: Cost,
    /// The zone it is cast from this way.
    pub from: ZoneKind,
    /// Its own text names that zone: warp's "You may cast this card from your hand for its warp cost" (CR 702.185a). A way
    /// from the hand that names no zone (Force of Will's, evoke's, web-slinging's) is an alternative cost that may be paid
    /// whatever zone a rule or permission has the spell cast from (CR 118.9, 601.3); one that names the hand is not open to a
    /// card cast from anywhere else, as a warped card cast from exile later is not (CR 702.185a).
    pub only_from: bool,
    /// Exiled instead of going anywhere else as it leaves the stack (flashback, CR 702.34a).
    pub exile_after: bool,
    pub condition: Option<Condition>,
    /// It may be cast this way whenever its caster could cast an instant, whatever the card's own type (sneak, CR 702.190a:
    /// "any time you could cast an instant during your declare blockers step"). `condition` says when that is.
    pub instant_speed: bool,
    /// A permanent spell cast this way enters tapped and attacking the same player, planeswalker or battle as the creature
    /// its cost returned to its owner's hand was attacking (sneak, CR 702.190b, 506.3, 508.4). What that creature attacked
    /// is kept on the spell as the cost is paid, in its `StackItem::subject`.
    pub enters_attacking: bool,
    /// Which keyword's way this is.
    pub kind: WayKind,
}

#[derive(Clone, Copy)]
pub struct Kicker {
    pub cost: ManaCost,
    /// Payable any number of times.
    pub multi: bool,
    /// A waterbend cost (CR 701.67a-b): as it is paid, an untapped artifact or creature may be tapped for each generic mana of
    /// it, and for no other mana of the spell's total cost. Spirit Water Revival's "you may waterbend {6}".
    pub waterbend: bool,
}

/// One way of paying an additional cost that names a choice ("discard a card or pay 3 life", CR 601.2b): its text,
/// which the question offers it by, and what paying it costs.
#[derive(Clone, Copy)]
pub struct ExtraCost {
    pub text: &'static str,
    pub cost: Cost,
}

/// A spell or ability that divides a total among its targets as they are chosen (CR 601.2d): its first target slot is
/// asked for together with each target's share, in one question. The slot's `max` is how many targets it may have.
#[derive(Clone, Copy)]
pub struct Divide {
    /// The total to divide, or `TargetSpec::COUNTED` for the X announced.
    pub total: u32,
    /// The total as a function of the X announced, where it is not `total` itself: "if X is 6 or more, twice X damage
    /// divided instead" (Shatterskull Smashing).
    pub of_x: Option<fn(u16) -> u16>,
}

/// An optional additional cost that is not mana (CR 601.2b): teamwork's "tap any number of creatures you control with
/// total power N or more". Announced once, like a kicker, by choosing between paying it and not, before any target;
/// paid with the other costs. A spell that paid it has `paid` 1 (how many, for a cost that sacrifices one or more),
/// which is not `kicked`: a card with a kicker and an optional cost of this kind is not ambiguous.
#[derive(Clone, Copy)]
pub struct OptionalCost {
    pub cost: Cost,
}

/// One mode of a modal spell or ability (CR 700.2). Its target slots are each one target (CR 700.2c); `cost` is what
/// choosing it adds to the total cost, for spree (CR 702.172a) and tiered (CR 702.183a).
#[derive(Clone, Copy)]
pub struct ModeDef {
    pub text: &'static str,
    pub targets: &'static [TargetSpec],
    pub effect: Effect,
    pub cost: ManaCost,
}

/// Modes chosen as a spell is cast or an ability activated (CR 601.2b, 602.2b, 700.2): between `min` and `max` of
/// them, each at most once unless `repeat`, run in printed order as it resolves (CR 700.2d). Each chosen mode beyond the
/// first costs `escalate` more (CR 702.120a): mana, a discard, life.
#[derive(Clone, Copy)]
pub struct ModalDef {
    pub modes: &'static [ModeDef],
    pub min: u8,
    pub max: u8,
    pub escalate: Option<Cost>,
    /// "You may choose the same mode more than once" (CR 700.2d): Mystic Confluence, Fiery Confluence. Each choice is
    /// an instance of its own, with targets of its own, run in sequence.
    pub repeat: bool,
    /// Whether mode `i` may be chosen now, beyond its targets (CR 700.2): "choose one that hasn't been chosen" (Gollum,
    /// Riddle Master) reads the modes this object chose before from the event history (`Event::ModesChosen`). A
    /// triggered ability left with too few modes it may choose is removed as it would be put on the stack (CR 603.3c).
    pub allowed: Option<fn(&dyn Query, &Asker, u8) -> bool>,
    /// How many modes a spell chooses, `(min, max)`, instead of `min` to `max` when its optional additional cost is paid or it
    /// is kicked: "Choose one. If this spell's additional cost was paid, choose both instead" (Pyrrhic Strike), "If this
    /// spell was kicked, choose any number instead" (Inscription of Abundance, Inscription of Insight, Inscription of Ruin).
    /// The intention to pay is announced in the same step as the modes (CR 601.2b), and the number of modes is defined by
    /// it, so the cost is announced first and the modes after it. An optional additional cost (`CardDef::optional_cost`) is
    /// offered only when that many modes could be chosen (CR 700.2a). Only that cost is gated: a kicker is not yet (no card
    /// in content is a kicked modal spell), and gating it is owed with the first one.
    /// A spell's own: a modal triggered ability (Depth Defiler's) does not read it.
    pub when_paid: Option<(u8, u8)>,
}

impl ModalDef {
    /// How many bits a mode's count takes in a `StackItem::modes` mask, for a modal that repeats: counts up to 15, past the
    /// five the Seasons choose of one mode (CR 700.2d), for up to eight modes.
    const COUNT_BITS: usize = 4;
    const COUNT_MAX: u32 = (1 << Self::COUNT_BITS) - 1;

    /// Where mode `i`'s bit, or its count's bits for a modal that repeats, are in a mask: `None` past the mask's width.
    fn shift(&self, i: usize) -> Option<u32> {
        let at = if self.repeat { Self::COUNT_BITS * i } else { i };
        (at < 32).then_some(at as u32)
    }

    /// How many times mode `i` is chosen in a `StackItem::modes` mask. A mask holds a bit for each mode, or, for a
    /// modal that repeats, four bits for each: its count.
    pub fn times(&self, mask: u32, i: usize) -> u8 {
        let Some(at) = self.shift(i) else { return 0 };
        let width = if self.repeat { Self::COUNT_MAX } else { 1 };
        ((mask >> at) & width) as u8
    }

    /// The mask of mode `i` chosen once.
    pub fn once(&self, i: usize) -> u32 {
        self.with(0, i)
    }

    /// The mask of one more choice of mode `i` on top of `mask`. A count already at its most takes no more, and never
    /// carries into the next mode's.
    pub fn with(&self, mask: u32, i: usize) -> u32 {
        let Some(at) = self.shift(i) else { return mask };
        if !self.repeat {
            return mask | (1 << at);
        }
        if u32::from(self.times(mask, i)) >= Self::COUNT_MAX { mask } else { mask + (1 << at) }
    }

    /// How many modes a mask chooses, repeats included.
    pub fn chosen(&self, mask: u32) -> u8 {
        (0..self.modes.len()).map(|i| self.times(mask, i)).fold(0u8, u8::saturating_add)
    }
}

impl SpellDef {
    /// The spell ability of a modal spell: it has no effect of its own (`CardDef::modal` holds the modes).
    pub const MODAL: SpellDef = SpellDef { targets: &[], effect: no_effect };
}

/// A spell's own ability (CR 113.3a): what an instant or sorcery does.
#[derive(Clone, Copy)]
pub struct SpellDef {
    pub targets: &'static [TargetSpec],
    pub effect: Effect,
}

/// The most modes a modal spell may choose, worked out from the board as the spell is cast (CR 601.2b, 700.2): "choose one.
/// If you control a Wizard as you cast this spell, you may choose two instead". Asked once, as the modes are announced.
pub type ModeCount = fn(&dyn Query, &Asker) -> u8;

/// What the board was as a spell was cast, for its effect to read later: "if you controlled a Faerie as you cast this
/// spell" (Faerie Fencing), "if you controlled a Mount" (Steer Clear), "a modified creature" (Flame Discharge), "for each
/// modified creature you controlled" (Lethal Exploit), "the greatest power among creatures you controlled" (Lifestream's
/// Blessing). Asked once, of the spell (`Asker::source`) and its caster, as it becomes cast (CR 601.2i) - once its costs are
/// paid, so a permanent sacrificed or one a mana ability sacrificed while paying is gone - and kept in its `Cast` record of
/// the history (`history::Cast::fact`). What happens to the board afterwards changes nothing (the rulings). A copy of the
/// spell was not cast (CR 707.10), so it has no such record and no such fact: Draconic Roar's ruling, "the copy wasn't
/// cast, so whether you controlled a Dragon won't matter".
pub type CastFact = fn(&dyn Query, &Asker) -> u32;

/// An additional cost worked out from a spell's announced targets (`CardDef::cost_of_targets`, CR 601.2f).
pub type TargetedCost = fn(&dyn Query, &Asker, &[Target]) -> Cost;

/// The cost a card in a graveyard or a hand may be cast for by a static ability's say-so (CR 113.6, 118.9), or none when it
/// may not be: asked of the granting permanent (`Asker`), the card (as the face being cast) and the player casting it.
/// Underworld Breach's escape is the card's mana cost and exiling three other cards; mayhem is its mana cost for a card
/// discarded this turn; a way for nothing would be `Cost::NONE`, "without paying its mana cost".
///
/// Whether X is announced follows from the cost (CR 107.3a-b): a cost that includes the card's mana cost has its {X}, and
/// one that is neither the mana cost nor includes X ("without paying its mana cost") leaves 0 the only legal choice for X.
pub type GrantedCost = fn(&dyn Query, &Asker, ObjectId, PlayerId) -> Option<Cost>;

/// A way of casting cards from graveyards, or from a hand, that a permanent gives (CR 604, 118.9).
#[derive(Clone, Copy)]
pub struct GrantedWay {
    /// What the way is called, which a scenario's `Ability:` names it by and a menu row shows beside the cast.
    pub text: &'static str,
    pub cost: GrantedCost,
    /// The zone of the caster's own the way casts cards from (CR 601.3): their graveyard (Lurrus, escape, mayhem) or their
    /// hand (a way given to cards in a hand, as web-slinging would be). Another zone's rule or permission does not open it.
    /// An alternative cost for spells cast from wherever (Omniscience's, Dracogenesis's) is a `SpellAlternative` instead.
    pub from: ZoneKind,
    /// The way also lets a land card in the graveyard be played (CR 305.1) rather than cast: `cost` is asked of the land
    /// card too, and any `Some` allows the play (the land has no cost). It uses the land drop, and the history marks the
    /// use as it marks a cast's, so "once during each of your turns" is shared between the play and the casts. Serra Paragon.
    pub lands: bool,
    /// "If you do, it gains [ability]": the index in the granting card's `delayed` abilities of the ability the permanent
    /// resulting from the play or the cast has from then on (CR 400.7a, 113.6). Serra Paragon's exile-and-gain-2-life.
    pub gains: Option<u8>,
    /// The way is flashback the permanent gives the card ("each instant and sorcery card in your graveyard has flashback",
    /// CR 702.34a): a spell cast this way is exiled instead of put anywhere else any time it would leave the stack, whether
    /// it resolves, is countered or is returned to a hand, and whether or not the permanent is still there. Iroh, Grand
    /// Lotus; Lier, Disciple of the Drowned and Kess's kin that give flashback as a static ability would use it.
    pub flashback: bool,
    /// The way is a permission and no alternative cost (CR 601.3): "you may cast a permanent spell with mana value 2 or less
    /// from your graveyard" (Lurrus of the Dream-Den, Serra Paragon), its `cost` the face's own mana cost. The card is cast
    /// by it as it would be by any permission - for its mana cost or an alternative cost another object gives or the card
    /// has (CR 118.9: Primal Prayers' energy, evoke), every additional cost owed - rather than only for `cost`.
    pub permission: bool,
}

impl GrantedWay {
    /// A way that casts only and grants nothing, for a card's file to start from.
    pub const CAST: GrantedWay =
        GrantedWay { text: "", cost: |_, _, _, _| None, from: ZoneKind::Graveyard, lands: false, gains: None, flashback: false, permission: false };
}

/// What a static ability lets a spell be cast for rather than its mana cost (CR 118.9, `SpellAlternative::cost`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsteadOfManaCost {
    /// "Without paying its mana cost" (Dracogenesis, Omniscience): no mana, and X is 0 (CR 107.3b).
    WithoutPaying,
    /// "You may pay [cost] rather than pay the mana cost" (Jodah, Archmage Eternal's and Fist of Suns' {W}{U}{B}{R}{G},
    /// Rooftop Storm's {0}): that mana, and X is 0, as it is not the mana cost (CR 107.3b).
    Mana(ManaCost),
    /// "By paying {E} rather than paying their mana costs" (Primal Prayers): energy counters (CR 107.14), no mana, and X is 0
    /// (CR 107.3b).
    Energy(u8),
}

impl InsteadOfManaCost {
    /// The mana paid rather than the mana cost.
    pub const fn mana(self) -> ManaCost {
        match self {
            InsteadOfManaCost::WithoutPaying | InsteadOfManaCost::Energy(_) => ManaCost::ZERO,
            InsteadOfManaCost::Mana(mana) => mana,
        }
    }

    /// The whole of what is paid rather than the mana cost: its mana and its energy.
    pub const fn cost(self) -> Cost {
        match self {
            InsteadOfManaCost::Energy(n) => Cost { energy: n, ..Cost::NONE },
            other => Cost::mana(other.mana()),
        }
    }
}

/// A static ability of a permanent that gives an alternative cost to spells a player casts (CR 118.9, 601.2b, 604.1):
/// "you may cast Dragon spells without paying their mana costs" (Dracogenesis), "you may pay {W}{U}{B}{R}{G} rather than pay
/// the mana cost for spells that you cast" (Jodah, Archmage Eternal; Fist of Suns), "you may pay {0} rather than pay the mana
/// cost for Zombie creature spells you cast" (Rooftop Storm), "you may cast spells from your hand without paying their mana
/// costs" (Omniscience, `applies` asking for the hand).
///
/// It is a way of paying and no permission (CR 601.3): it stands beside every cast the player may already make of the face
/// it judges - from a hand, a graveyard (Yawgmoth's Will, the card's own say-so, aftermath), exile (an Adventure, a
/// permanent's permission, a "you may cast it" effect), a library's top (a permanent's rule) or as an effect resolves "you
/// may cast that card" (CR 608.2g) - and offers it for this cost instead of the mana cost, at the same time. It is the one
/// alternative cost of that cast (CR 118.9a), so it is not offered where the cast already is for one: flashback, a
/// permission's own "without paying" or "rather than", life for its mana value, a card's own alternative cost (evoke,
/// bestow, overload), a way a permanent grants. Additional costs (kicker, a permission's "in addition", CR 118.9d) and what
/// raises or lowers the total (CR 601.2f) apply to it as to the mana cost; X is 0 (CR 107.3b). A card with no mana cost may
/// be cast for it (CR 118.6a). It functions while the permanent is on the battlefield with its abilities.
#[derive(Clone, Copy)]
pub struct SpellAlternative {
    /// What the cast is called on the menu (and what a scenario's `Ability:` names it by): "Without paying its mana cost".
    pub text: &'static str,
    /// Whether `player` may cast `spell` for it (CR 118.9): asked of the permanent (`Asker`, its controller "you"), with the
    /// spell as the face that would be cast (a Room's door, an Adventure, a back face through a view of that face alone,
    /// CR 709.3a, 715.3a, 712.8a), in the zone it is cast from, before it moves (CR 601.2a). "Spells you cast" asks
    /// `player == asker.controller`; "Dragon spells", the face's subtypes.
    pub applies: fn(&dyn Query, &Asker, ObjectId, PlayerId) -> bool,
    /// What is paid rather than the mana cost.
    pub cost: InsteadOfManaCost,
    /// "If you cast a spell this way, you may cast it as though it had flash" (Primal Prayers, CR 307.5, 702.8a): a cast for
    /// this cost may be made whenever its caster could cast an instant, by whatever lets the card be cast from where it is;
    /// what forbids casting at such a time (Teferi, Time Raveler) still does (CR 101.2).
    pub flash: bool,
}

/// A static ability of a permanent that lets a player cast cards in exile (CR 601.3, 604.1): "you may cast spells from among
/// cards exiled with this" (Rona, Disciple of Gix; Kheru Mind-Eater), "during your turn, you may cast creature spells from
/// among cards you own exiled with this creature by removing three counters from among creatures you control in addition to
/// paying their other costs" (Dawnhand Dissident). It is a permission, not a way of paying: the card is cast for its mana
/// cost or any alternative cost it has (CR 601.2b, 118.9), as its own Adventure or Omen too where `allows` says so, at the
/// time its type allows (Dawnhand Dissident's ruling: "normally ... only during your main phase while the stack is empty"),
/// and `additional` is owed on top of whatever it is cast for (CR 118.8, 601.2f). It functions while the permanent is on the
/// battlefield and has its abilities; a cast begun by it goes on if the permanent leaves while it is paid for (the same
/// card's ruling: sacrificed to pay an additional cost, or destroyed in response).
#[derive(Clone, Copy)]
pub struct ExileCast {
    /// What a cast by it is called on the menu, and on the card in exile it may cast.
    pub text: &'static str,
    /// Whether `player` may cast `card`, a card in exile, by it now: asked of the permanent (`Asker`, its controller "you"),
    /// with the card as the spell it would be (its Adventure or Omen through a view of that face, CR 715.3a, 720.3a), so
    /// "creature spells" refuses a creature card's instant Adventure.
    pub allows: fn(&dyn Query, &Asker, ObjectId, PlayerId) -> bool,
    /// The additional cost owed on each cast by it (CR 118.8); `Cost::NONE` for none.
    pub additional: Cost,
    /// "And mana of any type can be spent to cast that spell" (CR 609.4b): any mana pays any symbol of what a cast by it
    /// costs (`ManaCost::paid_with_any_type`), as a play permission's `PlayGrant::any_type` pays. Null Summoner's threshold ability.
    pub any_type: bool,
}

/// A static ability of a permanent that gives cards in a hand miracle (CR 702.94a): "each instant and sorcery card in your
/// hand has miracle {2}" (Lorehold, the Historian), "nonland cards in your hand have miracle {0}" (Molecule Man), "each
/// enchantment card in your hand has miracle; its miracle cost is equal to its mana cost reduced by {4}" (Aminatou, Veil
/// Piercer). It functions while the permanent is on the battlefield and has its abilities (CR 113.6, 613.1f); a card it
/// gives miracle to has that miracle ability as long as it does, the revealing as it is drawn included, and the linked
/// triggered ability, once it has triggered, is independent of the permanent (CR 113.7a, 607.5).
#[derive(Clone, Copy)]
pub struct GrantedMiracle {
    /// What the choice between several miracle abilities calls this one (`Label::Miracle`), worded with the permanent's name.
    pub text: &'static str,
    /// Whether the card, in a hand, has this miracle ability: asked of the granting permanent (`Asker`, its controller
    /// "you").
    pub has: fn(&dyn Query, &Asker, ObjectId) -> bool,
    /// Its miracle cost before `less`, for a card or the face of it being cast: the whole card as it is revealed, and the
    /// half, Adventure or face being cast as the linked ability resolves (CR 709.3a, 715.3a, 712.8a: only that face is
    /// evaluated). None is a cost based on the mana cost of a card that has none, which can't be paid (CR 118.6). An {X} in
    /// it is announced as the card is cast (CR 107.3a).
    pub cost: fn(&dyn Query, CardId) -> Option<ManaCost>,
    /// Generic mana the miracle cost is that cost reduced by ("its mana cost reduced by {4}"), 0 for none: part of what the
    /// cost is, so before anything increases it (CR 601.2f); generic mana only, never below nothing, and what the cost's
    /// own generic mana cannot take comes off X's once X is announced (CR 118.7a). See `reduced_by`.
    pub less: u8,
}

/// One miracle ability a card in a hand has (CR 702.94a): its own (`granter` none) or one a permanent gives it, named by the
/// granting card and the grant's index in its `granted_miracles`, which is what the linked triggered ability carries
/// (`StackItem::data`), since it is linked to that ability alone (CR 607.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Miracle {
    pub granter: Option<(CardId, u8)>,
    /// Its cost for the whole card, as it is revealed; none for a cost that can't be paid (CR 118.6).
    pub cost: Option<ManaCost>,
}

impl Miracle {
    /// The linked triggered ability's data: 0 for the card's own miracle, else the granting card's id plus one and the index.
    pub fn data(&self) -> [u32; 2] {
        self.granter.map_or([0; 2], |(card, i)| [card.0 as u32 + 1, i as u32])
    }

    /// The miracle ability a linked triggered ability's data names, for the card it was revealed with.
    pub fn of_data(q: &dyn Query, card: ObjectId, data: [u32; 2]) -> Option<Miracle> {
        if data[0] == 0 {
            return q.card(q.obj(card).card).miracle.map(|cost| Miracle { granter: None, cost: Some(cost) });
        }
        let granter = CardId((data[0] - 1) as u16);
        let grant = q.card(granter).granted_miracles.get(data[1] as usize)?;
        Some(Miracle { granter: Some((granter, data[1] as u8)), cost: (grant.cost)(q, q.obj(card).card).map(|c| reduced_by(c, grant.less).0) })
    }

    /// Its cost for casting `face` of the card (CR 709.3a, 715.3a, 712.8a): the card's own miracle cost whatever face is
    /// cast, a granted one as the grant reads that face.
    pub fn cost_as(&self, q: &dyn Query, face: CardId) -> Option<ManaCost> {
        match self.granter {
            None => self.cost,
            Some((granter, i)) => {
                q.card(granter).granted_miracles.get(i as usize).and_then(|grant| (grant.cost)(q, face).map(|c| reduced_by(c, grant.less).0))
            }
        }
    }

    /// The ability, named for a question about `card` (the card's id).
    pub fn of(&self, card: CardId) -> MiracleOf {
        match self.granter {
            None => MiracleOf { card, granter: card, index: MiracleOf::OWN },
            Some((granter, index)) => MiracleOf { card, granter, index },
        }
    }
}

/// A cost reduced by `less` generic mana (CR 118.7a): its generic mana comes off first, never below nothing, and colored mana
/// never; what is left of the reduction is taken off what an {X} in the cost adds once X is announced, and is 0 for a cost
/// with no {X}. The mana value of the spell is not changed (CR 202.3).
pub fn reduced_by(cost: ManaCost, less: u8) -> (ManaCost, u8) {
    let off = less.min(cost.generic);
    (ManaCost { generic: cost.generic - off, ..cost }, if cost.x > 0 { less - off } else { 0 })
}

/// A named miracle ability's cost (CR 702.94a) for casting `face` of its card, reduced as its grant says, read from the card
/// table alone: the card's own miracle cost, or the granting card's grant read for that face; with what is left of the
/// reduction for X (`reduced_by`). None for a cost that can't be paid (CR 118.6).
pub fn miracle_cost_and_x_less(q: &dyn Query, miracle: MiracleOf, face: CardId) -> Option<(ManaCost, u8)> {
    if miracle.is_own() {
        q.card(miracle.card).miracle.map(|cost| (cost, 0))
    } else {
        let grant = q.card(miracle.granter).granted_miracles.get(miracle.index as usize)?;
        (grant.cost)(q, face).map(|cost| reduced_by(cost, grant.less))
    }
}

/// A named miracle ability's cost for casting `face` of its card, reduced as its grant says (`miracle_cost_and_x_less`).
pub fn miracle_cost(q: &dyn Query, miracle: MiracleOf, face: CardId) -> Option<ManaCost> {
    miracle_cost_and_x_less(q, miracle, face).map(|(cost, _)| cost)
}

/// Every miracle ability a card in its owner's hand has (CR 702.94a): its own first, then those the permanents on the
/// battlefield give it, in the battlefield's order.
pub fn miracles(q: &dyn Query, card: ObjectId) -> Vec<Miracle> {
    let mut all: Vec<Miracle> = q.card(q.obj(card).card).miracle.map(|cost| Miracle { granter: None, cost: Some(cost) }).into_iter().collect();
    for &id in q.state().battlefield() {
        let o = q.obj(id);
        let granter = o.copied_card().unwrap_or(o.card);
        let def = q.card(granter);
        // A permanent that has lost all its abilities gives nothing (CR 613.1f).
        if def.granted_miracles.is_empty() || q.lost_all_abilities(id) {
            continue;
        }
        let asker = Asker { source: id, controller: o.controller };
        for (i, grant) in def.granted_miracles.iter().enumerate() {
            if (grant.has)(q, &asker, card) {
                all.push(Miracle { granter: Some((granter, i as u8)), cost: (grant.cost)(q, q.obj(card).card).map(|c| reduced_by(c, grant.less).0) });
            }
        }
    }
    all
}

/// Whether `player` can't draw `n` more cards right now (CR 121.2b, 121.3, 614.17): a static ability of a permanent on the
/// battlefield with its abilities says so (`CardDef::cant_draw`). With `n` of 1 it is about the card about to be drawn,
/// which then is not drawn and which no replacement effect may replace (CR 614.17c, Spirit of the Labyrinth's ruling); with
/// more, about an effect that offers the player the choice to draw that many, which they can't take (CR 121.2b: "you may
/// draw two cards" under "can't draw more than one card each turn"), the same as a choice to draw one they can't (CR 121.3).
pub fn cant_draw(q: &dyn Query, player: PlayerId, n: u32) -> bool {
    q.state().battlefield().iter().any(|&id| {
        let o = q.obj(id);
        // The definition its abilities come from: the card it copies, or the face that is up (CR 712.8d-f).
        let def = match o.copied_card() {
            Some(c) => q.card(c),
            None => {
                let front = q.card(o.card);
                match front.back {
                    Some(back) if o.transformed => back,
                    _ => front,
                }
            }
        };
        // A permanent that has lost all its abilities has none of these rules (CR 613.1f).
        !def.cant_draw.is_empty() && !q.lost_all_abilities(id) && {
            let asker = Asker { source: id, controller: o.controller };
            def.cant_draw.iter().any(|rule| rule(q, &asker, player, n))
        }
    })
}

/// A static ability that says whether a player may cast spells right now from a kind of zone (CR 101.2, 601.3): false
/// forbids it, for every spell cast from that zone, a cast an effect offers during resolution too. "Your opponents can't
/// cast spells during your turn" (Voice of Victory) ignores the zone; "your opponents can't cast spells from anywhere other
/// than their hands" (Drannith Magistrate) reads it.
pub type CastRule = fn(&dyn Query, &Asker, PlayerId, ZoneKind) -> bool;

/// A static ability that forbids casting spells with certain qualities (CR 101.2, 601.3): whether `player` may cast `spell`,
/// false forbidding it. "Noncreature spells with mana value 4 or greater can't be cast" (Gaddock Teeg), "noncreature
/// spells with {X} in their mana costs can't be cast" (Gaddock Teeg). It reads the spell's characteristics through the
/// query: a card that is to be cast is seen as the face it would be cast as (CR 601.3e, 709.3a, 715.3a, 712.8a), with X
/// as 0 (CR 202.3e), and asked again of the spell on the stack once its proposal is complete (CR 601.6), with X as
/// announced; a cast no longer allowed then is illegal and taken back (CR 733). Judged before the proposal with X as 0, a
/// quality that announcing X could lift (CR 601.3a) would be refused too early: no card forbids by such a quality.
pub type SpellRule = fn(&dyn Query, &Asker, PlayerId, ObjectId) -> bool;

/// A static ability that limits who may block whom (CR 509.1b): asked, while the object that has it is on the battlefield,
/// of every possible block - the blocker and the attacker, either of which may be the object itself, or neither; false
/// forbids the block. "This creature can't block or be blocked by non-Spirit creatures" (Wan Shi Tong's Spirits),
/// "Creatures you control with power 2 or less can't be blocked by creatures with power 3 or greater" (Delney, Streetwise
/// Lookout).
pub type BlockRule = fn(&dyn Query, ObjectId, ObjectId, ObjectId) -> bool;

/// A static ability that requires a creature to block a particular attacker if able (CR 509.1c): asked, while the object that
/// has it is on the battlefield with its abilities, of each block the restrictions allow - the object as `source`, then the
/// blocker and the attacker; true makes that block a requirement. "All creatures able to block enchanted creature do so"
/// (Lure: every creature able to block the creature it enchants has a requirement to block it). The defending player's
/// declaration must obey as many requirements as any declaration that obeys every restriction could (CR 509.1c).
pub type BlockRequirement = fn(&dyn Query, ObjectId, ObjectId, ObjectId) -> bool;

/// A static ability that says a creature can't attack (CR 508.1c), usually unless something holds: asked, while the object
/// that has it is on the battlefield with its abilities, of each creature that could otherwise be declared an attacker (the
/// object itself or another), with the object as `source`; false keeps the creature from attacking at all. "Delirium - This
/// creature can't attack or block unless there are four or more card types among cards in your graveyard" (Patchwork
/// Beastie); Sea Serpent's and Dandân's "can't attack unless defending player controls an Island" read the opponent, which in a
/// duel is the one defending player whatever the creature attacks. It is judged as attackers are declared: a creature whose
/// condition fails afterwards stays attacking (CR 506.4).
pub type AttackRule = fn(&dyn Query, ObjectId, ObjectId) -> bool;

/// A static ability that makes attacking cost something (CR 508.1d, 508.1h): "creatures can't attack you unless their
/// controller pays {2} for each creature they control that's attacking you" (Propaganda, Ghostly Prison, Windborn Muse),
/// "creatures can't attack you or planeswalkers you control unless their controller pays {1} for each of those creatures"
/// (Baird, Steward of Argive, Archon of Absolution). Asked, while its source is on the battlefield and has its abilities,
/// of each creature that may be declared an attacker and each player, planeswalker or battle it may attack: the cost of that
/// one attack, or `None` when this ability asks nothing of it. Every such cost of every such ability is added up (two
/// Ghostly Prisons ask {4} a creature) and locked in once the attackers are declared (CR 508.1h); the attacking player may
/// activate mana abilities and pays it all (CR 508.1i-j), or declares again. A cost is never a requirement: a creature
/// that attacks each combat if able need not attack where every attack open to it costs something (CR 508.1d).
/// `cantrip_mechanics::statics` has the shared ones.
pub type AttackCostRule = fn(&dyn Query, &Asker, ObjectId, Target) -> Option<ManaCost>;

/// What a [`TargetingRule`] changes about targeting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetingRuleKind {
    /// The creatures it names "can be the targets of spells and abilities as though they didn't have hexproof" (CR 702.11b,
    /// 609.4): hexproof, granted before or after, does not stop targeting them. Shroud and protection still do.
    IgnoreHexproof,
    /// "Ward abilities of those creatures don't trigger" (CR 702.21a): a ward ability of the object it names does not trigger
    /// when it becomes a target.
    SuppressWard,
}

/// A static ability that changes targeting rules for the objects it names (CR 604, 609.4), while its source is on the
/// battlefield: `applies` is asked of the source and of an object that is or would be a target, and sees the layers as they
/// are before the abilities layer. "Creatures your opponents control can be the targets of spells and abilities as though they
/// didn't have hexproof. Ward abilities of those creatures don't trigger" (Nowhere to Run).
#[derive(Clone, Copy)]
pub struct TargetingRule {
    pub kind: TargetingRuleKind,
    pub applies: fn(&dyn LayerQuery, ObjectId, ObjectId) -> bool,
}

/// A static ability that gives players a keyword ability (CR 702.18a "This permanent or player can't be the target of
/// spells or abilities", 702.11c), while its source is on the battlefield with its abilities: `applies` is asked of the
/// source and a player. Shroud and hexproof are the keywords a player can have; "You have shroud" (True Believer, Ivory
/// Mask) and "You have hexproof" (Leyline of Sanctity) print it. `cantrip_mechanics::statics` has the shared ones.
#[derive(Clone, Copy)]
pub struct PlayerKeywordRule {
    pub keyword: Keyword,
    pub applies: fn(&dyn LayerQuery, ObjectId, PlayerId) -> bool,
}

/// A static ability that gives a player protection from a player (CR 702.16k), while its source is on the battlefield with
/// its abilities: `applies` is asked of the source, the player who would have the protection, and the player it would be
/// from, and sees the layers as they are before the abilities layer. The protected player can't be targeted by spells or
/// abilities that player controls, can't be enchanted by Auras that player controls, and damage from sources that player
/// controls (or owns, while no one controls them) is prevented; nothing else changes - an effect that targets no one still
/// applies to them. "Protection from each of your opponents" is a protection from each opponent (CR 702.16i): Absolute
/// Virtue prints it.
#[derive(Clone, Copy)]
pub struct PlayerProtectionRule {
    pub applies: fn(&dyn LayerQuery, ObjectId, PlayerId, PlayerId) -> bool,
}

/// A triggered ability that an event would trigger, as a [`TriggerRule`] or an [`AdditionalTrigger`] is asked about it: its
/// source (a permanent, or a card in another zone whose ability functions there), the player who would control it, and its
/// definition.
#[derive(Clone, Copy)]
pub struct Triggering<'a> {
    pub source: ObjectId,
    pub controller: PlayerId,
    pub def: &'a TriggeredDef,
    /// It is an ability of a permanent: its source is on the battlefield, or was there as it last existed for an ability
    /// that looks back (CR 603.10a). What "a triggered ability of a permanent you control" and "abilities of permanents your
    /// opponents control" ask. False for a card in another zone, an emblem (CR 114.4), and a delayed triggered ability,
    /// which is no ability the object has (CR 603.2d, 603.7).
    pub permanent: bool,
    /// It looks back in time on the event it triggers on (CR 603.10, `CardTable::looks_back`): what stops it triggering or
    /// makes it trigger again is what was on the battlefield just before the event.
    pub looks_back: bool,
}

impl Triggering<'_> {
    /// Whether it is "a triggered ability of a permanent [`player`] controls" (CR 603.2d): the permanent as it is now, or as it
    /// last existed for an ability that looks back.
    pub fn of_a_permanent_controlled_by(&self, player: PlayerId) -> bool {
        self.permanent && self.controller == player
    }
}

/// A static ability that stops abilities from triggering (CR 603.2, 603.2g, 604.1): "Creatures entering don't cause abilities
/// to trigger" (Torpor Orb, Tocatli Honor Guard, Hushwing Gryff), "Artifacts and creatures entering ..." (Doorkeeper Thrull),
/// "Creatures entering or dying ..." (Hushbringer), "Permanents entering don't cause abilities of permanents your opponents
/// control to trigger" (Elesh Norn, Mother of Machines). Asked, with the rule's own asker, of one event and one triggered
/// ability that event matched; true stops that ability triggering on that event, and it is not put on the stack nor counted
/// against "triggers only N times each turn". Another event of the same batch still triggers it.
///
/// Which rules are asked is the core's (CR 603.10): for an ability that triggers on the board as it is after the event (an
/// enters ability, CR 603.6a), the permanents on the battlefield after it, so a rule entering with the permanent applies
/// (the rulings: "If Torpor Orb and a creature enter the battlefield at the same time ..."); for a leaves-the-battlefield
/// ability, which looks back (CR 603.10a), those on the battlefield before it, so one leaving with the dying creature applies
/// too. What an event is about is judged by the rule through `Query`: an entering permanent as it is on the battlefield, its
/// continuous effects applied (the rulings' Opalescence and March of the Machines), a creature that died as it last was there
/// (`Query::chars` of the old object). Replacement effects and "as [this] enters" (CR 614.1c, 614.12) are not triggered
/// abilities and are never asked about.
pub type TriggerRule = fn(&dyn Query, &Asker, &Event, &Triggering) -> bool;

/// A static ability that makes triggered abilities trigger an additional time (CR 603.2d): "If a permanent entering causes a
/// triggered ability of a permanent you control to trigger, that ability triggers an additional time" (Elesh Norn, Mother of
/// Machines, Yarok, Panharmonicon), "If a creature dying causes ..." (Teysa Karlov), "If a triggered ability of a Shaman or
/// another Wizard you control triggers ..." (Harmonic Prodigy). Asked as a [`TriggerRule`] is, of one event and one triggered
/// ability that event triggered and nothing stopped; true makes that ability trigger once more on that event. Each such
/// static ability adds one: two Elesh Norns make three (the rulings), and a "one or more" ability that triggers once for a
/// batch (CR 603.2c) triggers once more for each ability that applies to any of the batch's events. "That many times" is
/// each a separate triggering (CR 603.2d): each is put on the stack with its own modes and targets, and each is counted
/// against "triggers only N times each turn" (no numbered rule; CR 603.2h is "do this only once each turn"). Which static abilities are asked follows CR 603.10 as for
/// `TriggerRule` (one entering or dying with the event applies), and a delayed triggered ability is never asked about.
pub type AdditionalTrigger = fn(&dyn Query, &Asker, &Event, &Triggering) -> bool;

/// A replacement of a turn beginning (CR 614.10, 500.11): "if you would begin your turn while [condition], you may skip
/// that turn instead. If you do, [on_skip]". Time Vault. A skipped turn is not taken: its steps, its untap and its
/// number are not used, and the next player's turn begins in its place.
#[derive(Clone, Copy)]
pub struct TurnBeginRule {
    /// Whether it applies to `player`, whose turn would begin, judged on the board as it is.
    pub applies: fn(&dyn Query, &Asker, PlayerId) -> bool,
    /// What the effect does once its controller has chosen to skip the turn.
    pub on_skip: fn(&mut dyn Game, &Asker) -> Result<(), Suspend>,
    /// What its controller is asked, in the card's words.
    pub question: &'static Question,
    /// What it does, for the player who orders it with an effect that makes them skip the turn (CR 616.1): "you skip this
    /// turn and untap it".
    pub hint: &'static str,
}

/// The condition of a "for as long as" duration a card's spell or ability gives its effect (CR 611.2b), as
/// `Duration::AsLongAs`: Old Man of the Sea's "for as long as this creature remains tapped and that creature's power remains
/// less than or equal to this creature's power".
#[derive(Clone, Copy)]
pub struct Lasting {
    /// The condition after "for as long as", for the client: `{source}` is where the name of the effect's source goes.
    pub text: &'static str,
    /// Whether the condition is still true of the effect (its source and the object it affects), judged on the game now.
    pub holds: fn(&dyn Query, &ContinuousEffect) -> bool,
}

/// What a spell or ability may be forbidden to cause a player to do (`CauseRule`, CR 101.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CauseKind {
    /// Discard cards (CR 701.9).
    Discard,
    /// Sacrifice permanents (CR 701.21).
    Sacrifice,
    /// Search a library (CR 701.23): the one of `library`'s owner.
    Search { library: PlayerId },
}

/// A static ability that says whether a spell or ability may cause a player to discard cards, sacrifice permanents or search
/// a library (CR 101.2): true allows it. `causer` is the controller of the spell or ability, `player` who would do it.
/// "Spells and abilities your opponents control can't cause you to discard cards or sacrifice permanents."
pub type CauseRule = fn(&dyn Query, &Asker, PlayerId, PlayerId, CauseKind) -> bool;

/// A static ability that says some damage can't be prevented (CR 615.12): true for the damage a source is about to deal,
/// `combat` or not. "Combat damage that would be dealt by creatures you control can't be prevented."
pub type UnpreventableRule = fn(&dyn Query, &Asker, ObjectId, bool) -> bool;

/// A static ability that says a player can't gain life (CR 119.7): true for `player`. "Players can't gain life" (Sunspine
/// Lynx), "Your opponents can't gain life" (Knight of Dusk's Shadow). `cantrip_mechanics::statics` has both.
pub type LifeGainRule = fn(&dyn Query, &Asker, PlayerId) -> bool;

/// A static ability that limits the cards a player can draw (CR 121.2b, 121.3, 614.17): true when `player` can't draw `n` more
/// cards right now, the card about to be drawn being the first of them. "Each player can't draw more than one card each
/// turn" (Spirit of the Labyrinth), "each opponent can't draw more than one card each turn" (Narset, Parter of Veils;
/// Leovold, Emissary of Trest): `cantrip_mechanics::statics` has both. Asked by `cant_draw`.
pub type DrawRule = fn(&dyn Query, &Asker, PlayerId, u32) -> bool;

/// A static ability that says a player can't lose life (CR 119.8): true for `player`. Platinum Emperion's "your life total
/// can't change" is this and a `LifeGainRule` together.
pub type LifeLossRule = fn(&dyn Query, &Asker, PlayerId) -> bool;

/// A static ability that says a player doesn't lose the game (CR 104.3): true for `player` and the way they would lose.
/// "You can't lose the game" is true for every reason (Platinum Angel); "you don't lose the game for having 0 or less life"
/// for `LossReason::Life` alone (Phyrexian Unlife).
pub type LossRule = fn(&dyn Query, &Asker, PlayerId, crate::event::LossReason) -> bool;

/// A static ability that says a player can't win the game (CR 104.2b): true for `player`. Platinum Angel's "your opponents
/// can't win the game".
pub type WinRule = fn(&dyn Query, &Asker, PlayerId) -> bool;

/// A static ability under which damage dealt to `player` is dealt as though its source had infect (CR 702.90b): true for
/// that player now. Phyrexian Unlife.
pub type InfectDamageRule = fn(&dyn Query, &Asker, PlayerId) -> bool;

/// A static ability that says a permanent can't have counters of a kind put on it (CR 122.6, 101.2): true for `object` as
/// it would have them - `controller` its controller then, which for a permanent entering with counters is the player it
/// enters under - and `kind`. Melira, Sylvok Outcast; Solemnity.
pub type CounterBan = fn(&dyn Query, &Asker, ObjectId, PlayerId, CounterKind) -> bool;

/// The counters a player may get (CR 122.1), for `PlayerCounterBan`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerCounter {
    Poison,
    Energy,
    Experience,
}

/// A static ability that says a player can't get counters of a kind (CR 122.1, 101.2): true for `player` and `kind`.
/// Melira, Sylvok Outcast's "you can't get poison counters"; Solemnity's "players can't get counters".
pub type PlayerCounterBan = fn(&dyn Query, &Asker, PlayerId, PlayerCounter) -> bool;

/// Whether a static ability makes this creature assign combat damage equal to its toughness rather than its power
/// (CR 510.1a): given the rule's own asker.
pub type DamageByToughnessRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether a static ability keeps the damage marked on this permanent through the cleanup step (CR 514.2): given the
/// rule's own asker. "Damage isn't removed from this creature during cleanup steps" (Ancient Adamantoise); "from creatures
/// your opponents control" (Patient Zero, Uthgardt Fury) and "from creatures" (Case of the Market Melee) are the same kind.
pub type DamageStaysRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether a static ability untaps this permanent during each untap step of a player other than the ability's controller
/// (CR 502.3, 611.3a), given the rule's own asker, read before anything untaps: "untap this artifact" (Bender's Waterskin) is
/// true of its source, "untap all permanents you control" (Seedborn Muse) of every permanent its controller controls.
pub type OthersUntapRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether an ability of this permanent asks of the mana spent where it came from (CR 106.3): whether mana that `source`
/// produced is told apart from other mana for it, given the rule's own asker. "Using mana produced by [this]" (Barracks of the
/// Thousand) tells apart its own; "mana from a Treasure" (Kalain, Reclusive Painter) would tell apart a Treasure's.
pub type ManaSourceRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether a static ability keeps this planeswalker on the battlefield with 0 loyalty: it isn't put into its owner's
/// graveyard by the state-based action of CR 704.5i. Given the rule's own asker and the planeswalker. "Planeswalkers you
/// control aren't put into their owners' graveyards for having 0 loyalty" (Sanctum Lurker). Only that state-based action
/// is changed: a planeswalker that is destroyed or has its loyalty abilities' costs to pay is judged as always.
pub type ZeroLoyaltyRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether a static ability says the "legend rule" doesn't apply to this legendary permanent (CR 704.5j), given the rule's
/// own asker and the permanent: "The 'legend rule' doesn't apply" (Mirror Gallery, every permanent). Only that state-based
/// action is changed.
pub type LegendRuleExemption = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// Whether a static ability lets this creature attack as though it didn't have defender (CR 702.3b, 609.4): given the
/// rule's own asker and the creature. Demon Wall's "as long as this creature has a counter on it, it can attack as
/// though it didn't have defender"; Rolling Stones' "Wall creatures can attack as though they didn't have defender" and
/// High Alert's "creatures you control" are the same kind. It changes only whether the creature may attack: the creature
/// keeps defender for every other purpose (CR 609.4).
pub type AttackDespiteDefenderRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// A payment a "spend mana as though it were mana of any color" rule is asked about (CR 609.4b): the player paying, what the
/// cost is for - the permanent whose ability is activated, the spell, the source of an "unless pays" - and whether it is
/// an activation cost. `source` is `None` when the question is whether the player may do so for every payment whatever it
/// is for (Mycosynth Lattice's "players may spend mana as though it were mana of any color"), which the payments' advice
/// asks once.
#[derive(Clone, Copy, Debug)]
pub struct Spending {
    pub player: PlayerId,
    pub source: Option<ObjectId>,
    pub activating: bool,
}

/// Whether a static ability lets a player spend mana as though it were mana of any color for a payment (CR 609.4b), given
/// the rule's own asker and the payment: Agatha's Soul Cauldron's "you may ... to activate abilities of creatures you
/// control", Mycosynth Lattice's "players may ...". It changes how the cost may be paid, not the cost
/// (`ManaCost::paid_with_any_color`).
pub type SpendAnyColorRule = fn(&dyn Query, &Asker, &Spending) -> bool;

/// Whether a static ability lets its controller activate the abilities of this creature as though it had haste (CR 302.6):
/// given the rule's own asker and the creature whose ability is activated.
pub type ActivateAsThoughHasteRule = fn(&dyn Query, &Asker, ObjectId) -> bool;

/// A static ability that lets an ability that can only be activated at sorcery speed (a loyalty ability, CR 606.3) be
/// activated whenever its activator could cast an instant: asked of the ability's source, its index in the source's
/// computed list and the activator.
pub type InstantActivationRule = fn(&dyn Query, &Asker, ObjectId, u8, PlayerId) -> bool;

/// A static ability that lets its controller cast some spells as though they had flash (CR 307.5, 702.8): asked of the spell
/// (a card in the zone it is cast from) and the player casting it. Sigarda's Aid's Auras and Equipment.
pub type FlashRule = fn(&dyn Query, &Asker, ObjectId, PlayerId) -> bool;

/// A static ability that says whether an activated ability may be activated at all (CR 602.5, 605): asked of the permanent
/// that has the rule, the object whose ability it would be (in whatever zone it functions from) and that ability. False
/// forbids it, mana abilities included. "Activated abilities of artifacts, creatures, and planeswalkers can't be
/// activated" (Clarion Conqueror); Null Rod's and Stony Silence's "of artifacts" is the same rule over fewer types.
pub type ActivationRule = fn(&dyn Query, &Asker, ObjectId, &ActivatedDef) -> bool;

/// A permanent's standing rule about what its controller may play (CR 305.2, 305.1).
#[derive(Clone, Copy, Debug)]
pub enum PlayRule {
    /// "You may play an additional land on each of your turns" (CR 305.2): Icetill Explorer.
    ExtraLandPlays(u8),
    /// "You may play any number of lands on each of your turns" (CR 305.2): Fastbond.
    AnyNumberOfLandPlays,
    /// "You may play lands from your graveyard" (CR 305.1): Icetill Explorer.
    LandsFromGraveyard,
    /// "You may play lands and cast spells from the top of your library. If you cast a spell this way, pay life equal
    /// to its mana value rather than pay its mana cost" (CR 305.1, 601.3, 118.9): Bolas's Citadel.
    LibraryTop,
    /// "You may cast [spells that pass the filter] from the top of your library" (CR 601.3), for their mana cost: Mystic Forge's
    /// artifact and colorless spells, Traveling Chocobo's Bird spells. Lands are not played from there. The filter is asked
    /// with the permanent whose rule it is and its controller (`Asker`), so that a creature type it names is read through
    /// that permanent's words (CR 612), and the card on top.
    LibraryTopSpells(fn(&dyn Query, &Asker, ObjectId) -> bool),
    /// "You may play lands from the top of your library" (CR 305.1): Case of the Locked Hothouse, solved. Bolas's Citadel's
    /// `LibraryTop` includes it.
    LibraryTopLands,
    /// "You may look at the top card of your library any time" (CR 401.2): the player sees it whenever they could look, which
    /// the host shows them. Bolas's Citadel, Mystic Forge, Case of the Locked Hothouse (solved).
    LookAtLibraryTop,
    /// "As long as [condition], [rules]": the rules apply only while the condition holds, asked with the permanent as source
    /// and its controller. A Case's "Solved - [static ability]" means "As long as this Case is solved, [static ability]"
    /// (CR 702.169b): `cantrip_mechanics::solved::play_rules`.
    While(Condition, &'static [PlayRule]),
    /// "You have no maximum hand size" (CR 402.2): Proft's Eidetic Memory.
    NoMaximumHandSize,
}

/// What else may pay a spell's total cost besides mana (CR 601.2f-h): convoke (CR 702.51a), delve (CR 702.66a) and
/// improvise (CR 702.126a). Characteristics of the card, generated from its Oracle text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PayWith {
    pub convoke: bool,
    pub delve: bool,
    pub improvise: bool,
}

impl PayWith {
    pub const NONE: PayWith = PayWith { convoke: false, delve: false, improvise: false };

    pub const fn any(self) -> bool {
        self.convoke || self.delve || self.improvise
    }
}

/// What a card in an opening hand may do before the game begins (CR 103.6a): "begin the game with it on the battlefield". The
/// players decide in turn order, the starting player first, after mulligans.
#[derive(Clone, Copy)]
pub struct OpeningHandAction {
    /// Whether this player may take it: Gemstone Caverns's "if you're not the starting player".
    pub condition: Condition,
    /// The counter it enters with.
    pub counter: Option<(CounterKind, u8)>,
    /// "If you do, exile this many cards from your hand."
    pub exile_from_hand: u8,
}

/// A card: characteristics from the Oracle data, behavior as code.
#[derive(Clone, Copy)]
pub struct CardDef {
    pub name: &'static str,
    /// Ordered subtype words that name an unnamed token (CR 111.4, 612.2a). None for a card, explicitly named token
    /// or copy exception; a token's creating text changes these name words, not any literal name.
    pub token_name: Option<&'static [Subtype]>,
    /// Whether these are a card's characteristics, generated from its Oracle data (a face of a card included), rather than
    /// a token's or an emblem's written in code: only a card's name may be chosen as a card name (CR 201.4, 114.5). The
    /// oracle sets it; everything built on `CardDef::BLANK` leaves it false. A dungeon (CR 309), written in code, is a
    /// card too: `CardTable::card_named` counts it.
    pub oracle: bool,
    pub cost: ManaCost,
    /// Whether it has a mana cost at all (CR 118.6): a card without one cannot be cast for it.
    pub has_cost: bool,
    /// Its color indicator or defined colors, where they are not its cost's.
    pub colors: Option<ColorSet>,
    pub types: TypeSet,
    pub supertypes: Supertypes,
    pub subtypes: SubtypeSet,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub loyalty: Option<i16>,
    /// A battle's printed defense (CR 310.4a): it enters with that many defense counters (CR 310.4b).
    pub defense: Option<i16>,
    pub keywords: Keywords,
    /// Its printed "hexproof from [quality]" abilities (CR 702.11d): a keyword ability, so it is part of its copiable values
    /// and is lost as keywords are (CR 702.11e). A color word in it is a word of its text (CR 612.2): a text-changing effect
    /// changes the color.
    pub hexproof_from: HexproofFrom,
    /// Its printed "protection from [color]" abilities (CR 702.16a): White Knight's "protection from black". A keyword
    /// ability, so part of its copiable values and lost as keywords are; its color word is a word of its text (CR 612.2),
    /// which a text-changing effect changes.
    pub protection: ColorSet,
    /// Convoke, delve, improvise: what besides mana may pay its total cost.
    pub pay_with: PayWith,
    pub spell: Option<SpellDef>,
    pub abilities: &'static [AbilityDef],
    /// Alternative ways to cast it; way n is `ways[n - 1]`, way 0 its mana cost.
    pub ways: &'static [WayDef],
    /// Overload (CR 702.96a): the cost it may be cast for instead of its mana cost, changing every "target" in its text to
    /// "each". Cast that way (`WAY_OVERLOAD`) it has no targets, and its effect reads `item.way` to reach everything the
    /// target would have been chosen from.
    pub overload: Option<ManaCost>,
    /// An optional additional cost paid once, or any number of times (kicker, multikicker, squad: CR 702.33,
    /// 702.157).
    pub kicker: Option<Kicker>,
    /// An optional additional cost that is not mana (teamwork), announced as a kicker is. The question offers it by
    /// `labels`: the first is "Cast without it", the second the cost's text.
    pub optional_cost: Option<OptionalCost>,
    /// Its spell's first target slot divides a total among its targets (CR 601.2d), as Pyrokinesis's damage does.
    pub divide: Option<Divide>,
    /// Its modes, chosen as it is cast (CR 601.2b, 700.2); its `spell` is `SpellDef::MODAL`.
    pub modal: Option<&'static ModalDef>,
    /// The most modes the board lets it choose as it is cast, where that is more than `modal.max` (CR 700.2): Flame of Anor.
    pub modes_up_to: Option<ModeCount>,
    /// What the board was as it was cast, which its effect reads from the history (CR 601.2i): `CastFact`.
    pub cast_fact: Option<CastFact>,
    /// An additional cost that depends on the targets announced (CR 601.2b-c, 601.2f): "as an additional cost to cast this
    /// spell, collect evidence X, where X is the total mana value of the permanents this spell targets" (Urgent Necropsy).
    /// Asked of the spell (`Asker::source`) and its caster with the targets in slot order once they are chosen, and added to
    /// the total cost determined then; a total its caster can't pay takes the cast back (CR 601.2h).
    pub cost_of_targets: Option<TargetedCost>,
    /// Its targets are chosen after the kicker and X are announced, because what may be targeted depends on them
    /// (CR 601.2b-c): Bloodchief's Thirst's kicked target.
    pub late_targets: bool,
    /// Delayed abilities created as it is cast for a way, with no stack item of their own (CR 608.3g, 603.7a): a pair
    /// of the way (as `ways[n - 1]`) and the index in `delayed`. The ability triggers at the beginning of the next end
    /// step, aimed at the permanent the spell becomes: dash's "return it to its owner's hand" (CR 702.109a), blitz's.
    pub end_step_riders: &'static [(u8, u8)],
    /// A cost it always adds to its mana cost (CR 118.8, 601.2b): Tinker's "sacrifice an artifact".
    pub additional_cost: Option<Cost>,
    /// An additional cost of which one of these is paid, chosen as it is cast (CR 601.2b): Bitter Triumph.
    pub additional_choice: &'static [ExtraCost],
    /// Its own entering replacements, applied as it enters from anywhere (CR 614.12).
    pub enters: Option<Enters>,
    /// Its "as this transforms into [this face]" ability (CR 712.20), on the face it is printed on: applied as the permanent
    /// turns over to this face. See `AsTransforms`.
    pub as_transforms: Option<AsTransforms>,
    /// Abilities its effects create to trigger later (CR 603.7), as `AbilityRef::delayed(card, n)`.
    pub delayed: &'static [AbilityDef],
    /// Its prepare spell (CR 722): what a copy cast while it is prepared is a copy of.
    pub prepare: Option<&'static CardDef>,
    /// Madness (CR 702.35): the cost it may be cast for when discarded. A discard puts it into exile instead of the
    /// graveyard, and a triggered ability then lets its owner cast it for this or puts it into the graveyard.
    pub madness: Option<ManaCost>,
    /// Miracle (CR 702.94): the cost it may be cast for when revealed as the first card drawn in a turn.
    pub miracle: Option<ManaCost>,
    /// "You may cast this card from your graveyard using its bestow ability" (CR 702.103): the bestowed Aura is cast from
    /// the graveyard for its bestow cost, and the card is not otherwise cast from there.
    pub bestow_from_graveyard: bool,
    /// A restriction on when this spell may be cast at all (CR 601.3): "Cast this spell only before the combat damage
    /// step". Judged for every way of casting it, from any zone.
    pub cast_only_if: Option<Condition>,
    /// A split card (CR 709): its two halves, each a card of its own that is the spell on the stack when it is cast
    /// (CR 709.3). The card itself has both halves' characteristics together in every other zone (CR 709.4). The
    /// definitions must be in the card table (listed in the card's `EXTRA`).
    pub split: Option<(&'static CardDef, &'static CardDef)>,
    /// Fuse (CR 702.102): a split card that may be cast from its owner's hand as both halves at once (`WAY_FUSED`), the
    /// fused split spell, with the card's combined characteristics (CR 702.102b, 709.4) and both halves' mana costs (CR
    /// 702.102c). The core makes it from `split`: the left half's targets and then the right half's
    /// (`CardTable::spell_targets_of`), and as it resolves the left half's instructions and then the right half's (CR
    /// 702.102d). A fuse card needs nothing else.
    pub fuse: bool,
    /// Aftermath (CR 702.127a): this half may be cast only from its owner's graveyard, and is exiled when it leaves the
    /// stack.
    pub aftermath: bool,
    /// Reconfigure (CR 702.151): an Equipment creature with it may be attached to a creature (CR 301.5c).
    pub reconfigure: bool,
    /// Foretell (CR 702.143): the cost it is cast for from exile, on a later turn, once foretold.
    pub foretell: Option<ManaCost>,
    /// Suspend (CR 702.62).
    pub suspend: Option<SuspendDef>,
    /// Its Adventure (CR 715): a second set of characteristics it can be cast with from hand. The definition must be
    /// in the card table (listed in the card's `EXTRA`).
    pub adventure: Option<&'static CardDef>,
    /// Its Omen (CR 720): the alternative characteristics it may be cast with as an Omen instant or sorcery (CR 720.2,
    /// 720.3), the only ones it has on the stack (CR 720.3b), and shuffled into its owner's library as it resolves (CR
    /// 720.3d). The definition must be in the card table (listed in the card's `EXTRA`).
    pub omen: Option<&'static CardDef>,
    /// Bestow (CR 702.103): the Aura it is while it is bestowed, an Aura enchantment with "enchant creature" and the card's
    /// own mana cost (CR 702.103b: it keeps its other characteristics), which it is on the stack and on the battlefield until
    /// it ceases to be bestowed (CR 702.103e-f). The definition must be in the card table (listed in the card's `EXTRA`),
    /// and gives the Aura everything it has that the creature does: its other abilities as well as "enchanted creature gets
    /// ...". It is cast for `bestow_cost`.
    pub bestow: Option<&'static CardDef>,
    /// The bestow cost (CR 702.103a), with `bestow`: an alternative cost (CR 118.9), paid rather than the mana cost, so the
    /// bestowed spell's mana value is still its mana cost's (CR 202.3).
    pub bestow_cost: ManaCost,
    /// Rebound (CR 702.88a): cast from its caster's hand and resolving, it is exiled instead of put into the
    /// graveyard, and its controller may cast it from exile at the beginning of their next upkeep (`REBOUND_ABILITY`).
    pub rebound: bool,
    /// Paradigm (CR 702.192a): its two spell abilities. The one given here (`cantrip_mechanics::paradigm::SPELL_ABILITY`) runs
    /// after the card's own effect and makes the delayed triggered ability; the core does the other, "exile this spell", as it
    /// leaves the stack having resolved.
    pub paradigm: Option<Effect>,
    /// Its standing rules about playing lands and cards from other zones, in force while it is on the battlefield.
    pub play_rules: &'static [PlayRule],
    /// Texts its questions offer as options (`Label::Card`).
    pub labels: &'static [&'static str],
    /// Its static abilities that change characteristics (CR 604, 613).
    pub statics: &'static [StaticDef],
    /// Its replacement effects (CR 614): "if this would happen, that happens instead". Functioning while it is on the
    /// battlefield, or in every zone for its own move (`ReplacementDef::own_everywhere`).
    pub replacements: &'static [ReplacementDef],
    /// The replacement effects its resolving spells and abilities make for a duration (CR 611.2, 614): `EffectKind::Replacement(n)`
    /// and `ReplacementRule(n)` name the nth. Not standing abilities: they function only while such an effect lasts.
    pub granted_replacements: &'static [GrantedReplacementDef],
    /// Its prevention effects (CR 615) of static abilities: "prevent all damage that would be dealt to [this]". Functioning
    /// while it is on the battlefield.
    pub preventions: &'static [PreventionDef],
    /// The prevention effects its resolving spells and abilities make for a duration (CR 611.2, 615): `EffectKind::Prevention`
    /// names the nth.
    pub granted_preventions: &'static [GrantedPreventionDef],
    /// "Each player can't cast more than this many spells each turn" (CR 101.2), while it is on the battlefield; 0 for
    /// no such limit. Archon of Emeria, High Noon, Rule of Law.
    pub cast_limit: u8,
    /// Its static abilities that make other things cost more or less (CR 601.2f), while it is on the battlefield.
    pub cost_changes: &'static [CostChange],
    /// A spell's own cost changes (CR 601.2f): "this spell costs {1} less to cast for each ..." (affinity, Embercleave).
    /// They work wherever the spell is cast from, and the spell is their `source`.
    pub own_cost_changes: &'static [CostChange],
    /// The cost changes its resolving spells and abilities make for a duration (CR 611.2c, 601.2f): `EffectKind::SpellCostChange`
    /// names the nth. Not standing abilities: they function only while such an effect lasts.
    pub lasting_cost_changes: &'static [LastingCostChange],
    /// Its static abilities that give the spells a player casts convoke, delve or improvise (CR 601.2f-h), while it is on the
    /// battlefield.
    pub grants_pay_with: &'static [PayWithGrant],
    /// Its static abilities that let a player pay another mana cost rather than an activated ability's (CR 118.9), while
    /// it is on the battlefield.
    pub ability_alternatives: &'static [AbilityAlternative],
    /// Ways it gives cards in graveyards to be cast, while it is on the battlefield (CR 604, 118.9).
    pub granted_ways: &'static [GrantedWay],
    /// Alternative costs it gives the spells a player casts, from wherever they are cast, while it is on the battlefield
    /// with its abilities (CR 118.9, 601.2b): Dracogenesis's "you may cast Dragon spells without paying their mana costs".
    pub spell_alternatives: &'static [SpellAlternative],
    /// Its static abilities that let a player cast cards in exile (CR 601.3, 604.1), while it is on the battlefield with its
    /// abilities: "you may cast spells from among cards exiled with this".
    pub exile_casts: &'static [ExileCast],
    /// Miracle it gives cards in its controller's hand, while it is on the battlefield (CR 702.94a, 113.6): Lorehold, the
    /// Historian's "each instant and sorcery card in your hand has miracle {2}".
    pub granted_miracles: &'static [GrantedMiracle],
    /// What it may do from an opening hand (CR 103.6a): begin the game on the battlefield.
    pub opening_hand: Option<OpeningHandAction>,
    /// Its back face (CR 711): the characteristics of the object when it is transformed. A double-faced card's
    /// back face is not a card of its own: nothing can be drawn or cast as it.
    pub back: Option<&'static CardDef>,
    /// Its back face is a land that may be played from where the card could be played as a land (CR 712.12, 305.1): a
    /// modal double-faced card. The land enters as the back face (CR 712.14a).
    pub back_land: bool,
    /// Its printed basic land types' mana, in printed order: the order its intrinsic mana abilities have (CR 305.6).
    pub basic_land_types: &'static [Mana],
    pub text: &'static str,
    /// What an Aura may enchant (CR 303.4a, 702.5): the one target of the Aura spell, and what a legal host is for as
    /// long as it is attached (CR 704.5m).
    pub enchant: Option<TargetSpec>,
    /// The enchant ability it gains when it becomes an Aura (CR 303.4, 702.5a, 613.1d), by an effect that adds the Aura subtype
    /// to it: Necromancy's \"enchant creature put onto the battlefield with Necromancy\". It is not an Aura spell, so it targets nothing.
    pub becomes_aura_enchant: Option<TargetSpec>,
    /// \"You may cast this spell as though it had flash\" (CR 307.5, 702.8): it may be cast whenever its caster could cast an instant.
    pub cast_as_though_flash: bool,
    /// Ascend (CR 702.131): while a permanent of this card is on the battlefield and its controller controls ten or more
    /// permanents, they get the city's blessing.
    pub ascend: bool,
    /// Storied (CR 702.195a): while a permanent of this card is on the battlefield and its controller controls three or
    /// more permanents that are artifacts, Sagas and/or legendary, they have an enduring story for the rest of the game.
    pub storied: bool,
    /// Start your engines! (CR 702.179a): a player who controls a permanent of this card and has no speed gets a speed
    /// of 1, as a state-based action (CR 704.5aa).
    pub start_your_engines: bool,
    /// These are what a face-down permanent is (CR 708.2): the copiable values the rule that turned it face down lists - a
    /// 2/2 creature for a manifested one (CR 701.40a, 708.2a), and with ward {2} for a cloaked one (CR 701.58a). An object
    /// whose copiable values are such a definition was turned face down before it entered (CR 708.3); its first ability
    /// turns it face up (`intrinsic::TURN_FACE_UP`). Only the rules' own definitions (`crate::intrinsic`) set it.
    pub face_down: bool,
    /// A permanent of this card whose controller sacrifices it at the beginning of the cleanup step when this says so (CR 514.1, 603.7):
    /// Necromancy's \"if you cast it any time a sorcery couldn't have been cast\".
    pub sacrifice_at_cleanup: Option<fn(&dyn Query, ObjectId) -> bool>,
    /// A token definition that other token definitions share a name with (three Golems that differ in a keyword): the
    /// card table keeps such definitions apart instead of merging definitions of one name.
    pub shares_name: bool,
    /// Splice onto Arcane (CR 702.47): the cost paid to add this card's spell effect to an Arcane spell as it is cast. The
    /// spell ability it adds must have no targets.
    pub splice: Option<ManaCost>,
    /// A dungeon card (CR 309): for each room, the rooms the arrows from it lead to (CR 309.5a); a room with none is
    /// the bottommost. The room's ability is the ability of the same index, and its name and text the label.
    pub dungeon: &'static [&'static [u8]],
    /// Its static abilities that forbid casting spells (CR 101.2).
    pub cast_rules: &'static [CastRule],
    /// Its static abilities that forbid casting spells with certain qualities (CR 101.2, 601.3), while it is on the battlefield
    /// with its abilities.
    pub spell_rules: &'static [SpellRule],
    /// Its static abilities that limit blocking (CR 509.1b), while it is on the battlefield.
    pub block_rules: &'static [BlockRule],
    /// Its static abilities that require creatures to block (CR 509.1c), while it is on the battlefield.
    pub block_requirements: &'static [BlockRequirement],
    /// Its static abilities that keep creatures from attacking (CR 508.1c), while it is on the battlefield.
    pub attack_rules: &'static [AttackRule],
    /// Its static abilities that make attacking cost something (CR 508.1d, 508.1h), while it is on the battlefield.
    pub attack_costs: &'static [AttackCostRule],
    /// Its static abilities that let creatures attack as though they didn't have defender (CR 702.3b, 609.4), while it is
    /// on the battlefield with its abilities. Asked only of a creature with defender as attackers are declared.
    pub attacks_despite_defender: &'static [AttackDespiteDefenderRule],
    /// Its static abilities that change who can target what and whether ward triggers, while it is on the battlefield.
    pub targeting_rules: &'static [TargetingRule],
    /// Its static abilities that give players keyword abilities ("You have shroud", CR 702.18a), while it is on the
    /// battlefield with its abilities.
    pub player_keywords: &'static [PlayerKeywordRule],
    /// Its static abilities that give players protection from players ("You have protection from each of your opponents",
    /// CR 702.16k), while it is on the battlefield with its abilities.
    pub player_protection: &'static [PlayerProtectionRule],
    /// Its static abilities that make some damage unpreventable (CR 615.12).
    pub unpreventable_damage: &'static [UnpreventableRule],
    /// Its static abilities that stop players from gaining life (CR 119.7), while it is on the battlefield: the gain, the
    /// part of an exchange or a new life total that would raise the total, does not happen (CR 119.5).
    pub cant_gain_life: &'static [LifeGainRule],
    /// Its static abilities that stop players from losing life (CR 119.8), while it is on the battlefield with its abilities:
    /// the loss - damage's, a payment's, an effect's, a new life total's that would lower the total - does not happen, and a
    /// cost that pays more than 0 life can't be paid.
    pub cant_lose_life: &'static [LifeLossRule],
    /// Its static abilities that stop players from drawing cards (CR 121.2b, 614.17), while it is on the battlefield with its
    /// abilities: a draw that can't happen does not, and no replacement effect replaces it (CR 614.17c); a choice to draw
    /// that many can't be taken (CR 121.3). See `cant_draw`.
    pub cant_draw: &'static [DrawRule],
    /// Its static abilities that keep a player from losing the game (CR 104.3, 704.5a-c), while it is on the battlefield with
    /// its abilities: "you can't lose the game" (Platinum Angel), "you don't lose the game for having 0 or less life"
    /// (Phyrexian Unlife). A concession still loses (CR 104.3a), and nothing stops a draw (CR 104.4).
    pub loss_rules: &'static [LossRule],
    /// Its static abilities that keep a player from winning the game (CR 104.2b), while it is on the battlefield with its
    /// abilities: "your opponents can't win the game" (Platinum Angel). The last player left still wins (CR 104.2a).
    pub win_rules: &'static [WinRule],
    /// Its static abilities that make damage dealt to a player be dealt as though its source had infect (CR 702.90b, 120.3b),
    /// while it is on the battlefield with its abilities: Phyrexian Unlife's "as long as you have 0 or less life".
    pub infect_damage_to: &'static [InfectDamageRule],
    /// Its static abilities that keep counters from being put on permanents (CR 122.6, 101.2), while it is on the battlefield
    /// with its abilities: Melira, Sylvok Outcast's "creatures you control can't have -1/-1 counters put on them", Solemnity's
    /// "artifacts, creatures, enchantments, and lands can't have counters put on them". It entering with counters is
    /// counters being put on it too (CR 122.6).
    pub counter_bans: &'static [CounterBan],
    /// Its static abilities that keep players from getting counters (CR 122.1, 101.2), while it is on the battlefield with
    /// its abilities: Melira's "you can't get poison counters", Solemnity's "players can't get counters".
    pub player_counter_bans: &'static [PlayerCounterBan],
    /// "Each creature you control assigns combat damage equal to its toughness rather than its power" (CR 510.1a):
    /// Assault Formation, Doran, the Siege Tower.
    pub damage_by_toughness: &'static [DamageByToughnessRule],
    /// Its static abilities that keep damage on permanents during cleanup steps (CR 514.2), while it is on the battlefield
    /// with its abilities. An effect that removes all damage (regeneration, CR 701.19a) still removes it.
    pub damage_stays: &'static [DamageStaysRule],
    /// Its abilities that ask where the mana spent came from (CR 106.3), while it is on the battlefield with its abilities:
    /// mana a rule tells apart is spent only as its payer chooses (CR 601.2h), never where other mana of its kind would do.
    pub mana_told_apart: &'static [ManaSourceRule],
    /// Its static abilities that keep planeswalkers with 0 loyalty on the battlefield (CR 704.5i), while it is on the
    /// battlefield with its abilities.
    pub zero_loyalty_stays: &'static [ZeroLoyaltyRule],
    /// Its static abilities that keep the "legend rule" (CR 704.5j) from applying to some permanents, while it is on the
    /// battlefield with its abilities.
    pub legend_rule_exempt: &'static [LegendRuleExemption],
    /// "You may spend mana as though it were mana of any color to activate abilities of creatures you control" (CR 609.4b):
    /// Agatha's Soul Cauldron; "Players may spend mana as though it were mana of any color": Mycosynth Lattice.
    pub spend_any_color: &'static [SpendAnyColorRule],
    /// "You may activate each of those abilities only once each turn" (CR 602.5b): how many of a permanent's instances of
    /// an activated ability are ones its static ability gave it with that limit (Locus of Enlightenment's of the exiled
    /// cards used to craft it, Mairsil, the Pretender's of the caged cards), asked of the permanent of this card as the
    /// ability would be activated. Each such instance may be activated once each turn; an instance of the same ability it
    /// has another way is not limited.
    pub granted_once_each_turn: Option<fn(&dyn Query, ObjectId, AbilityRef) -> usize>,
    /// "You may activate abilities of creatures you control as though those creatures had haste" (CR 302.6, 702.10c): a
    /// {T} in the cost of an activated ability of such a creature may be paid though it came under its controller's control
    /// this turn. It gives no haste: the creature still can't attack. Shang-Chi, Master of Kung Fu and Thousand-Year
    /// Elixir.
    pub activate_as_though_haste: &'static [ActivateAsThoughHasteRule],
    /// Its static abilities that forbid spells and abilities to cause discards or sacrifices (CR 101.2).
    pub cause_rules: &'static [CauseRule],
    /// Its replacements of its controller's turn beginning (CR 614.10).
    pub turn_begin: &'static [TurnBeginRule],
    /// The conditions of the "for as long as" durations its spells and abilities give their effects (CR 611.2b), which a
    /// `Duration::AsLongAs` names by index.
    pub lasting: &'static [Lasting],
    /// "You may choose not to untap this [permanent] during your untap step" (CR 502.3): its controller is asked, as their
    /// untap step untaps their permanents, whether it untaps. Old Man of the Sea, Rubinia Soulsinger, Seasinger print it.
    pub may_not_untap: bool,
    /// "Untap [this permanent / all permanents you control of a kind] during each other player's untap step" (CR 502.3,
    /// 611.3a): while it is on the battlefield with its abilities, in each untap step of a player other than its
    /// controller, the tapped permanents not controlled by the active player that a rule is true of untap, at the same
    /// time as the active player's, and with no choice (ruling). "Doesn't untap during its controller's untap step" and
    /// "you may choose not to untap" speak of that player's own untap step, so they don't stop it; a stun counter still
    /// replaces the untap (CR 122.1d). "This": Bender's Waterskin, Victory Chimes, Endbringer, Thousand Moons Infantry;
    /// "all permanents you control": Seedborn Muse; of a kind: Prophet of Kruphix, Unwinding Clock, Drumbellower.
    pub untap_in_others_untap_steps: &'static [OthersUntapRule],
    /// Its static abilities that allow sorcery-speed abilities at instant speed (CR 606.3).
    pub instant_activation: &'static [InstantActivationRule],
    /// Its static abilities that let spells be cast as though they had flash (CR 307.5): "You may cast Aura and Equipment spells
    /// as though they had flash" (Sigarda's Aid). The battlefield is searched for these only when something is cast.
    pub flash_rules: &'static [FlashRule],
    /// Its static abilities that forbid activating abilities (CR 602.5): Clarion Conqueror, Null Rod, Stony Silence.
    pub activation_rules: &'static [ActivationRule],
    /// Its static abilities that stop abilities from triggering (CR 603.2), while it is on the battlefield: Torpor Orb.
    pub trigger_rules: &'static [TriggerRule],
    /// Its static abilities that make triggered abilities trigger an additional time (CR 603.2d), while it is on the
    /// battlefield: Elesh Norn, Mother of Machines, Panharmonicon, Teysa Karlov.
    pub additional_triggers: &'static [AdditionalTrigger],
    /// Gift (CR 702.174): an optional additional cost of promising an opponent a gift, given as it resolves, before its
    /// other effects (an instant or sorcery), or when it enters (a permanent).
    pub gift: Option<GiftDef>,
}

/// A card's gift (CR 702.174a): which gift it is, for the promise to name, and what giving it does to the chosen
/// player. `cantrip_mechanics::gift` has one for each kind.
#[derive(Clone, Copy)]
pub struct GiftDef {
    pub kind: Gift,
    /// Gives the gift to the chosen opponent (CR 702.174d-i).
    pub give: fn(&mut dyn Game, PlayerId) -> Result<(), Suspend>,
}

impl CardDef {
    pub const BLANK: CardDef = CardDef {
        name: "",
        token_name: None,
        oracle: false,
        cost: ManaCost::ZERO,
        has_cost: true,
        colors: None,
        types: TypeSet::NONE,
        supertypes: Supertypes { basic: false, legendary: false, snow: false },
        subtypes: SubtypeSet::NONE,
        power: None,
        toughness: None,
        loyalty: None,
        defense: None,
        keywords: Keywords::NONE,
        hexproof_from: HexproofFrom::NONE,
        protection: ColorSet::NONE,
        pay_with: PayWith::NONE,
        spell: None,
        abilities: &[],
        ways: &[],
        kicker: None,
        overload: None,
        optional_cost: None,
        divide: None,
        modal: None,
        modes_up_to: None,
        cast_fact: None,
        cost_of_targets: None,
        end_step_riders: &[],
        late_targets: false,
        additional_cost: None,
        additional_choice: &[],
        enters: None,
        as_transforms: None,
        delayed: &[],
        prepare: None,
        madness: None,
        miracle: None,
        bestow_from_graveyard: false,
        cast_only_if: None,
        split: None,
        fuse: false,
        aftermath: false,
        reconfigure: false,
        foretell: None,
        suspend: None,
        adventure: None,
        omen: None,
        bestow: None,
        bestow_cost: ManaCost::ZERO,
        rebound: false,
        paradigm: None,
        play_rules: &[],
        labels: &[],
        statics: &[],
        replacements: &[],
        granted_replacements: &[],
        preventions: &[],
        granted_preventions: &[],
        cast_limit: 0,
        cost_changes: &[],
        own_cost_changes: &[],
        lasting_cost_changes: &[],
        grants_pay_with: &[],
        ability_alternatives: &[],
        granted_ways: &[],
        spell_alternatives: &[],
        exile_casts: &[],
        granted_miracles: &[],
        opening_hand: None,
        back: None,
        back_land: false,
        basic_land_types: &[],
        text: "",
        enchant: None,
        becomes_aura_enchant: None,
        cast_as_though_flash: false,
        ascend: false,
        storied: false,
        start_your_engines: false,
        face_down: false,
        sacrifice_at_cleanup: None,
        shares_name: false,
        splice: None,
        dungeon: &[],
        cast_rules: &[],
        spell_rules: &[],
        block_rules: &[],
        block_requirements: &[],
        attack_rules: &[],
        attack_costs: &[],
        attacks_despite_defender: &[],
        targeting_rules: &[],
        player_keywords: &[],
        player_protection: &[],
        unpreventable_damage: &[],
        cant_gain_life: &[],
        cant_lose_life: &[],
        cant_draw: &[],
        loss_rules: &[],
        win_rules: &[],
        infect_damage_to: &[],
        counter_bans: &[],
        player_counter_bans: &[],
        damage_by_toughness: &[],
        damage_stays: &[],
        mana_told_apart: &[],
        zero_loyalty_stays: &[],
        legend_rule_exempt: &[],
        spend_any_color: &[],
        granted_once_each_turn: None,
        activate_as_though_haste: &[],
        cause_rules: &[],
        turn_begin: &[],
        lasting: &[],
        may_not_untap: false,
        untap_in_others_untap_steps: &[],
        instant_activation: &[],
        flash_rules: &[],
        activation_rules: &[],
        trigger_rules: &[],
        additional_triggers: &[],
        gift: None,
    };

    pub fn colors(&self) -> ColorSet {
        self.colors.unwrap_or_else(|| self.cost.colors())
    }

    /// The spells printed in an inset frame that it may be cast as instead of itself (CR 715.2, 720.2): its Adventure, cast
    /// with `WAY_ADVENTURE`, and its Omen, cast with `WAY_OMEN`.
    pub fn inset_faces(&self) -> impl Iterator<Item = (u8, &'static CardDef)> {
        [self.adventure.map(|a| (crate::WAY_ADVENTURE, a)), self.omen.map(|o| (crate::WAY_OMEN, o))].into_iter().flatten()
    }

    /// The face it is cast as by an inset way (`WAY_ADVENTURE` or `WAY_OMEN`), if it has one.
    pub fn inset_face(&self, way: u8) -> Option<&'static CardDef> {
        self.inset_faces().find(|&(w, _)| w == way).map(|(_, f)| f)
    }

    /// A Room (CR 709.5): a split card whose halves share a type line, each half a door.
    pub fn is_room(&self) -> bool {
        self.split.is_some() && self.subtypes.has(Subtype::Room)
    }

    /// What this door of a Room has that the engine does not take from its door (CR 709.5), if anything: the Room permanent
    /// has an unlocked door's abilities (`layers::doors`) and play rules, and nothing else of its text. Everything a card
    /// does while on the battlefield besides those - static abilities, replacement and prevention effects, an entering rule,
    /// and each standing rule a card lists - is read off the Room card, so it would be absent; a triggered ability that
    /// functions outside the battlefield would be too, as the doors' abilities are given on the battlefield alone; and a
    /// keyword the door printed would be the whole card's whether or not the door is unlocked. `CardTable::new` refuses a
    /// door that has any. What a door does as a spell (its cost, targets and effect, its own cost changes) is its own.
    pub fn door_unsupported(&self) -> Option<&'static str> {
        let refused = [
            (!self.statics.is_empty(), "static abilities"),
            (!self.replacements.is_empty(), "replacement effects"),
            (!self.preventions.is_empty(), "prevention effects"),
            (self.enters.is_some(), "an enters rule"),
            (self.keywords != Keywords::NONE, "keywords"),
            (self.abilities.iter().any(|a| matches!(a, AbilityDef::Triggered(t) if t.anywhere)), "a triggered ability that functions outside the battlefield"),
            (self.abilities.iter().any(|a| matches!(a, AbilityDef::Static(_))), "a static ability in its abilities"),
            (self.cast_limit > 0, "a cast limit"),
            (!self.cost_changes.is_empty(), "cost changes"),
            (!self.grants_pay_with.is_empty(), "convoke, delve or improvise it gives"),
            (!self.ability_alternatives.is_empty(), "alternative costs for abilities"),
            (!self.granted_ways.is_empty(), "ways it gives to cast cards"),
            (!self.spell_alternatives.is_empty(), "alternative costs it gives spells"),
            (!self.exile_casts.is_empty(), "casting from exile it allows"),
            (!self.granted_miracles.is_empty(), "miracle it gives"),
            (!self.cast_rules.is_empty(), "cast rules"),
            (!self.spell_rules.is_empty(), "spell rules"),
            (!self.block_rules.is_empty(), "block rules"),
            (!self.block_requirements.is_empty(), "block requirements"),
            (!self.attack_rules.is_empty(), "attack rules"),
            (!self.attack_costs.is_empty(), "attack costs"),
            (!self.attacks_despite_defender.is_empty(), "attacking despite defender"),
            (!self.targeting_rules.is_empty(), "targeting rules"),
            (!self.player_keywords.is_empty(), "player keywords"),
            (!self.player_protection.is_empty(), "player protection"),
            (!self.unpreventable_damage.is_empty(), "unpreventable damage"),
            (!self.cant_gain_life.is_empty(), "a life gain rule"),
            (!self.cant_lose_life.is_empty(), "a life loss rule"),
            (!self.cant_draw.is_empty(), "a draw rule"),
            (!self.loss_rules.is_empty(), "loss rules"),
            (!self.win_rules.is_empty(), "win rules"),
            (!self.infect_damage_to.is_empty(), "an infect damage rule"),
            (!self.counter_bans.is_empty(), "counter bans"),
            (!self.player_counter_bans.is_empty(), "player counter bans"),
            (!self.damage_by_toughness.is_empty(), "damage by toughness"),
            (!self.damage_stays.is_empty(), "a damage-stays rule"),
            (!self.mana_told_apart.is_empty(), "a mana source rule"),
            (!self.zero_loyalty_stays.is_empty(), "a zero-loyalty rule"),
            (!self.legend_rule_exempt.is_empty(), "a legend rule exemption"),
            (!self.spend_any_color.is_empty(), "a spend-as-any-color rule"),
            (!self.activate_as_though_haste.is_empty(), "activating as though with haste"),
            (!self.cause_rules.is_empty(), "cause rules"),
            (!self.turn_begin.is_empty(), "a turn-begin replacement"),
            (!self.untap_in_others_untap_steps.is_empty(), "untapping in others' untap steps"),
            (!self.instant_activation.is_empty(), "instant-speed activation rules"),
            (!self.flash_rules.is_empty(), "flash rules"),
            (!self.activation_rules.is_empty(), "activation rules"),
            (!self.trigger_rules.is_empty(), "trigger rules"),
            (!self.additional_triggers.is_empty(), "additional triggers"),
            (self.ascend || self.storied || self.start_your_engines, "a designation keyword"),
            (self.may_not_untap || self.sacrifice_at_cleanup.is_some() || self.granted_once_each_turn.is_some(), "a permanent's rule"),
            (!self.basic_land_types.is_empty(), "basic land types"),
        ];
        refused.into_iter().find(|&(has, _)| has).map(|(_, what)| what)
    }

    /// The doors of a Room that the unlocked designations `doors` name, left first (CR 709.5c).
    pub fn unlocked_doors(&self, doors: u8) -> impl Iterator<Item = &'static CardDef> {
        let (left, right) = self.split.map_or((None, None), |(l, r)| (Some(l), Some(r)));
        [left.filter(|_| doors & DOOR_LEFT != 0), right.filter(|_| doors & DOOR_RIGHT != 0)].into_iter().flatten()
    }

    /// The mana cost of a Room permanent with the unlocked designations `doors`: the combined costs of its unlocked
    /// doors, and none at all when neither is (CR 709.5, 709.4b).
    pub fn room_cost(&self, doors: u8) -> ManaCost {
        self.unlocked_doors(doors).fold(ManaCost::ZERO, |sum, half| sum.plus(&half.cost))
    }

    pub fn is(&self, t: CardType) -> bool {
        self.types.has(t)
    }

    /// The target slots of its spell (CR 601.2c): an instant's or sorcery's own, or the one an Aura spell has in its
    /// enchant ability (CR 303.4a).
    pub fn spell_targets(&'static self) -> &'static [TargetSpec] {
        match (&self.spell, &self.enchant) {
            (Some(spell), _) => spell.targets,
            (None, Some(enchant)) => std::slice::from_ref(enchant),
            _ => &[],
        }
    }
}

/// Every card the game knows, built once per process and shared, never copied.
pub struct CardTable {
    pub cards: Vec<&'static CardDef>,
    /// Which cards have a triggered ability that functions outside the battlefield, so the rest are never looked at
    /// there.
    pub anywhere: Vec<bool>,
    /// Which cards have static abilities: the layers look for sources only among them.
    pub statics: Vec<bool>,
    /// Which cards have a control-changing static ability (CR 613.1b, `Layer::Control`): what tells whether control could
    /// change without working out the static sources.
    pub control_statics: Vec<bool>,
    /// Which cards could have a triggered ability that may look back in time (CR 603.10, `TriggeredDef::may_look_back`) of
    /// their own: one they print, one they list to give (`CardDef::delayed`), any a static ability of theirs could give
    /// themselves, or, for a Room, one a door prints (CR 709.5; `CardTable::new`) - the objects a look-back pass needs as
    /// they were just before the event.
    pub look_back: Vec<bool>,
    /// The triggered abilities whose condition says what its event can't show (`read_zone_words`), by ability, sorted; and which
    /// cards have one, so the rest are never searched.
    pub zone_words: Vec<(AbilityRef, ZoneWords)>,
    pub zone_word_cards: Vec<bool>,
    /// Which cards have a static ability of layer 6 that affects other objects (CR 613.1f), which may give one an ability.
    pub ability_statics: Vec<bool>,
    /// Whether any card has a static ability that functions from a graveyard, so graveyards are looked in at all.
    pub graveyard_statics: bool,
    /// Whether any card has a static ability that functions from exile (cipher's, CR 702.99a), so exile is looked in.
    pub exile_statics: bool,
    /// Which cards have play rules, and whether any does: land plays look for them only when one might.
    pub play_rules: Vec<bool>,
    pub any_play_rules: bool,
    /// Which cards have replacement or prevention effects of static abilities (CR 614, 615), and whether any does: events
    /// look for sources only when one might be.
    pub replaces: Vec<bool>,
    pub any_replaces: bool,
    /// Whether any card limits the spells cast each turn.
    pub any_cast_limit: bool,
    /// Which cards have block rules (CR 509.1b): looked at only for the blocker and the attacker of a possible block.
    pub block_rules: Vec<bool>,
    /// Which cards have attack rules (CR 508.1c).
    pub attack_rules: Vec<bool>,
    /// Per card: it has a targeting rule. Read to find the few battlefield sources of one, never to ask the table.
    pub targeting_rules: Vec<bool>,
    /// Per card: it gives players keyword abilities (`CardDef::player_keywords`) or protection (`CardDef::player_protection`).
    /// Read to find the few battlefield sources of one.
    pub player_keywords: Vec<bool>,
    /// Which cards could have a triggered mana ability (CR 605.1b) of their own, as `look_back` says for look-back abilities:
    /// one they print (a Room's doors' included), one they list to give, or any a static ability of theirs could give
    /// themselves. With every other way an ability is given (`settle::could_have`), the permanents looked at when a
    /// permanent is tapped for mana.
    pub mana_triggers: Vec<bool>,
    /// Per card: the fused split spell's target specs of a split card with fuse (CR 702.102b: the left half's, then the right
    /// half's), made once as the table is.
    pub fused_targets: Vec<Option<&'static [TargetSpec]>>,
    /// Which cards have a state trigger (CR 603.8), printed, a Room's door's, or among the abilities they give (`CardDef::delayed`): looked for
    /// among the permanents as the game changes with no event (an effect beginning in the middle of a resolution).
    pub state_triggers: Vec<bool>,
    /// Which cards have a static ability that gives other objects abilities (layer 6, CR 613.1f): with one on the board, any
    /// permanent may have a state trigger its card does not print.
    pub ability_grants: Vec<bool>,
    /// Which cards have cost changes (CR 601.2f): the board is looked at for their sources, never for the card table's
    /// sake (a table holds every card, so "does any card" is always true).
    pub cost_changes: Vec<bool>,
    /// Which cards give spells convoke, delve or improvise (`CardDef::grants_pay_with`): their sources are found on the board.
    pub pay_with_grants: Vec<bool>,
    /// Which cards forbid activating abilities (CR 602.5): their sources are found on the board, as cost changes' are.
    pub activation_rules: Vec<bool>,
    /// Which cards stop abilities from triggering (CR 603.2) or make them trigger additional times (CR 603.2d): their sources
    /// are found on the board when something triggers.
    pub trigger_rules: Vec<bool>,
    /// Which cards give their controller a designation once the board allows it (ascend, CR 702.131b; storied, CR
    /// 702.195a): their sources are found on the board, and the board counted only for a player one of them works for.
    pub designators: Vec<bool>,
    /// Which cards have start your engines! (CR 702.179a): their permanents are found on the board for a player with no
    /// speed (CR 704.5aa).
    pub engines: Vec<bool>,
    /// The rules' own abilities (`crate::intrinsic`): what a basic land type gives a land (CR 305.6).
    pub intrinsic: CardId,
    /// What a manifested permanent is (CR 701.40a, 708.2a): its copiable values.
    pub face_down: CardId,
    /// What a cloaked permanent is (CR 701.58a): its copiable values.
    pub cloaked: CardId,
    /// What a face-down card outside the battlefield and the stack is (CR 406.3a): nothing.
    pub no_characteristics: CardId,
    /// A Room's two doors (CR 709.5), by the Room's id: the ids of its halves, whose abilities its permanent has while
    /// they are unlocked.
    pub halves: Vec<Option<(CardId, CardId)>>,
    /// A double-faced card's back face, by the front's id (CR 711).
    pub backs: Vec<Option<CardId>>,
    /// The front face of a back face, by the back's id: a decklist may name either face.
    pub fronts: Vec<Option<CardId>>,
    /// The Undercity dungeon card, which the initiative ventures into (CR 726.2), if the game has it.
    pub undercity: Option<CardId>,
    /// A card's characteristics when nothing acts on it (the layers' fast path), made on first use: `[0]` before the
    /// ability layer, `[1]` with the abilities the rules give lands and Sieges. A pure function of the card alone.
    pub printed: Vec<[std::sync::OnceLock<Box<Chars>>; 2]>,
    /// Per card: its name is a card's - some definition of that name is a card's (`CardDef::oracle`, or a dungeon) - so a player may
    /// choose it (CR 201.4: "a player may not choose the name of a token unless it's also the name of a card"). False
    /// for a name only a token or an emblem has.
    pub card_named: Vec<bool>,
}

/// What a triggered ability's trigger condition says about where its trigger event's object comes from, the part of a
/// zone-change trigger condition its event can't show (`CardTable::looks_back`), read from the ability's own words as the
/// table is made, so no card declares it: "from anywhere" (CR 603.6c) - not "from anywhere other than ...", which names the
/// zones it leaves out and is about leaving the others - and a card leaving a graveyard (CR 603.10a), "leave(s) ...
/// graveyard". The condition is the text up to the first comma that doesn't continue a list ("an artifact, creature, or
/// enchantment card"): what follows is the effect or an intervening "if".
pub fn read_zone_words(text: &str) -> ZoneWords {
    let text = text.to_ascii_lowercase();
    let mut segments = text.split(", ");
    let mut condition = segments.next().unwrap_or_default().to_string();
    for next in segments {
        if next.starts_with("or ") || next.starts_with("and ") || !next.contains(' ') {
            condition.push_str(", ");
            condition.push_str(next);
        } else {
            break;
        }
    }
    let from_anywhere = condition.match_indices("from anywhere").any(|(at, w)| !condition[at + w.len()..].starts_with(" other than"));
    let leaves_graveyard = condition.match_indices("leave").any(|(at, _)| condition[at..].contains("graveyard"));
    ZoneWords { from_anywhere, leaves_graveyard }
}

/// What a zone-change trigger condition says that its event can't show (`read_zone_words`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZoneWords {
    /// "Whenever [a card] is put into [a zone] from anywhere" (CR 603.6c): about where the object goes, so never a
    /// leaves-the-battlefield ability, even for an object put there from the battlefield (Planar Void).
    pub from_anywhere: bool,
    /// Its trigger event is a card leaving a graveyard (CR 603.10a): it looks back on a card put onto the battlefield from
    /// a graveyard too, which is otherwise an enters-the-battlefield event judged after it (CR 603.6a) (Hardened Academic).
    pub leaves_graveyard: bool,
}

/// Whether a triggered ability whose condition says `words` looks back in time (CR 603.10) as it triggers on `event`: it is
/// judged on the game as it was just before the event - which objects had it, what they looked like, who controlled them,
/// and what stopped or multiplied triggering (CR 603.2d) - rather than as it is after. The rules decide it from the trigger
/// event, never the card:
///
/// - a zone change from the battlefield is a leaves-the-battlefield ability's (CR 603.6c, 603.10a), a sacrifice's included,
///   unless the condition is "put into [a zone] from anywhere" (CR 603.6c);
/// - a card leaving a graveyard (CR 603.10a), unless the condition is about where it goes "from anywhere"; one put onto the
///   battlefield is an enters-the-battlefield event (CR 603.6a), looked back on only by a condition about the card leaving
///   the graveyard;
/// - an object all players can see put into a hand or a library (CR 603.10a), "from anywhere" or not: one from the
///   battlefield, a graveyard, the stack or the command zone, or from exile face up, or face down where every player in
///   the game may look at it (CR 406.3);
/// - a permanent phasing out (CR 603.10b), a permanent becoming unattached (CR 603.10c), a spell countered (CR 603.10e) and
///   a player losing the game (CR 603.10f).
pub fn looks_back(words: ZoneWords, q: &dyn Query, event: &Event) -> bool {
    match event {
        Event::ZoneChange { old, from, to, .. } => {
            let seen_by_all = || {
                [ZoneKind::Battlefield, ZoneKind::Graveyard, ZoneKind::Stack, ZoneKind::Command].into_iter().any(|z| from.is(z))
                    || (from.is(ZoneKind::Exile) && {
                        let o = q.obj(*old);
                        !o.face_down || q.state().apnap().all(|p| o.seen_by & (1 << p.0) != 0)
                    })
            };
            ((to.is(ZoneKind::Hand) || to.is(ZoneKind::Library)) && seen_by_all())
                || (!words.from_anywhere
                    && (from.is(ZoneKind::Battlefield)
                        || (from.is(ZoneKind::Graveyard) && (!to.is(ZoneKind::Battlefield) || words.leaves_graveyard))))
        }
        Event::PhasedOut { .. } | Event::Unattached { .. } | Event::Countered { .. } | Event::GameLost { .. } => true,
        _ => false,
    }
}

/// How a card is put onto the battlefield face down, which says what it is while it is (CR 708.2): manifested, a 2/2
/// creature (CR 701.40a); cloaked, a 2/2 creature with ward {2} (CR 701.58a). Either is turned face up for its card's
/// mana cost if it is a creature card (CR 701.40b, 701.58b).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceDown {
    Manifested,
    Cloaked,
}

impl CardTable {
    pub fn new(cards: impl IntoIterator<Item = &'static CardDef>) -> CardTable {
        let mut cards: Vec<&'static CardDef> = cards.into_iter().collect();
        cards.push(&crate::intrinsic::INTRINSIC);
        cards.push(&crate::intrinsic::FACE_DOWN);
        cards.push(&crate::intrinsic::CLOAKED);
        cards.push(&crate::intrinsic::NO_CHARACTERISTICS);
        // A back face is registered with its front (the first of a name wins, as for every card).
        let backs: Vec<&'static CardDef> = cards.iter().filter_map(|c| c.back).collect();
        cards.extend(backs);
        // Stable, and the first of a name kept: a card's own file comes before its characteristics-only definition.
        cards.sort_by(|a, b| a.name.cmp(b.name));
        // Only the same definition twice is merged: two tokens of one name (a Germ, a Golem) are two definitions,
        // and the registry already leaves out a characteristics-only card that has a file of its own.
        cards.dedup_by(|a, b| std::ptr::eq(*a, *b));
        // A static ability given in layer 6 applies from there on (CR 613.1f): once given, in layer 6 itself, and from 7a on.
        // One of an earlier layer would come into being after its layer was done, and would be quietly absent: refused.
        for card in &cards {
            if let Some(name) = card.token_name {
                assert!(!card.oracle, "{}: a card's name is literal", card.name);
                assert_eq!(ObjectName::Token { subtypes: name, words: Words::NONE }.to_string(), card.name, "token definition's name disagrees with CR 111.4");
                assert!(name.iter().all(|&s| card.subtypes.has(s)), "{}: token-name words are assigned subtypes", card.name);
            }
            for ability in card.abilities.iter().chain(card.delayed) {
                if let AbilityDef::Static(s) = ability {
                    assert!(s.layer >= Layer::Ability, "{}: a granted static ability applies in layer 6 or later (CR 613.1f)", card.name);
                }
            }
            // The layers find what a "loses all abilities" static took from among the permanents
            // (`layers::lost_to_statics`), and a characteristic-defining ability is no such ability (CR 604.3).
            for s in card.statics.iter().filter(|s| s.loses_all) {
                assert!(!s.everywhere && !s.own, "{}: a static ability that makes objects lose all abilities affects permanents", card.name);
            }
            // A later part of an effect (CR 613.6) follows the part before it, in a later layer, from the same zone: the
            // layers apply it to the objects that part applied to, which they know only once that part's layer is done.
            for (i, s) in card.statics.iter().enumerate().filter(|(_, s)| s.continues) {
                let before = i.checked_sub(1).map(|j| &card.statics[j]);
                assert!(
                    before.is_some_and(|b| b.layer < s.layer && b.from == s.from && b.everywhere == s.everywhere && !b.own && !s.own),
                    "{}: a static ability that continues an effect follows its earlier part, in a later layer (CR 613.6)",
                    card.name
                );
            }
            // A control-changing ability (CR 613.1b) gives control of permanents: the core asks it of each one on the
            // battlefield.
            for s in card.statics.iter().filter(|s| s.layer == Layer::Control) {
                assert!(!s.everywhere && !s.own && !s.continues, "{}: a control-changing static ability affects other permanents", card.name);
            }
            // A fused split spell is made by the core from its halves' targets and instructions alone (CR 702.102b-d): a half
            // with a choice or cost of its own as it is cast (CR 601.2b-f), or text the core would have to run another way,
            // would be cast fused without it, so such a card is refused rather than played wrong.
            if card.fuse {
                assert!(card.spell.is_none(), "{}: a fuse card's fused spell is made from its halves; it has no spell of its own", card.name);
                let Some((left, right)) = card.split else { panic!("{}: fuse is a split card's (CR 702.102a)", card.name) };
                for half in [left, right] {
                    let unbuilt = fused_half_unbuilt(half);
                    assert!(unbuilt.is_none(), "{}: its half {} has {}, which a fused cast does not make (CR 702.102)", card.name, half.name, unbuilt.unwrap_or(""));
                }
            }
            // And what one that sets land subtypes took (`layers::land_text_lost`): it affects lands on the battlefield.
            for s in card.statics.iter().filter(|s| s.sets_land_subtypes) {
                assert!(!s.everywhere && !s.own && s.layer == Layer::Type, "{}: a static ability that sets land subtypes affects permanents in layer 4", card.name);
            }
        }
        let anywhere = cards
            .iter()
            .map(|c| c.abilities.iter().any(|a| matches!(a, AbilityDef::Triggered(t) if t.anywhere)))
            .collect();
        let statics: Vec<bool> = cards.iter().map(|c| c.statics.iter().any(|s| !s.own)).collect();
        let control_statics: Vec<bool> = cards.iter().map(|c| c.statics.iter().any(|s| s.layer == Layer::Control)).collect();
        let looks_back = |a: &AbilityDef| matches!(a, AbilityDef::Triggered(t) if t.may_look_back());
        let look_back: Vec<bool> = cards
            .iter()
            .map(|c| {
                c.abilities.iter().any(looks_back)
                    || c.delayed.iter().any(looks_back)
                    || c.statics.iter().any(|s| s.own && s.layer == Layer::Ability)
            })
            .collect();
        // Read once, from each triggered ability's words (`read_zone_words`): a card's place in `cards` is its id.
        let mut zone_words: Vec<(AbilityRef, ZoneWords)> = Vec::new();
        for (id, card) in cards.iter().enumerate() {
            let listed = card.abilities.iter().enumerate().map(|(i, a)| (i as u8, a));
            let delayed = card.delayed.iter().enumerate().map(|(i, a)| (AbilityRef::DELAYED + i as u8, a));
            for (index, ability) in listed.chain(delayed) {
                if let AbilityDef::Triggered(t) = ability {
                    let words = read_zone_words(t.text);
                    if words != ZoneWords::default() {
                        zone_words.push((AbilityRef { card: CardId(id as u16), index }, words));
                    }
                }
            }
        }
        zone_words.sort_unstable_by_key(|&(a, _)| a);
        let mut zone_word_cards = vec![false; cards.len()];
        for &(a, _) in &zone_words {
            zone_word_cards[a.card.0 as usize] = true;
        }
        let ability_statics: Vec<bool> = cards.iter().map(|c| c.statics.iter().any(|s| !s.own && s.layer == Layer::Ability)).collect();
        let graveyard_statics = cards.iter().any(|c| c.statics.iter().any(|s| s.from == ZoneKind::Graveyard));
        let exile_statics = cards.iter().any(|c| c.statics.iter().any(|s| s.from == ZoneKind::Exile));
        let play_rules: Vec<bool> = cards.iter().map(|c| !c.play_rules.is_empty()).collect();
        let any_play_rules = play_rules.iter().any(|&s| s);
        let replaces: Vec<bool> = cards.iter().map(|c| !c.replacements.is_empty() || !c.preventions.is_empty()).collect();
        let any_replaces = replaces.iter().any(|&r| r);
        let any_cast_limit = cards.iter().any(|c| c.cast_limit > 0);
        let cost_changes: Vec<bool> = cards.iter().map(|c| !c.cost_changes.is_empty()).collect();
        let pay_with_grants: Vec<bool> = cards.iter().map(|c| !c.grants_pay_with.is_empty()).collect();
        let activation_rules: Vec<bool> = cards.iter().map(|c| !c.activation_rules.is_empty()).collect();
        let trigger_rules: Vec<bool> = cards.iter().map(|c| !c.trigger_rules.is_empty() || !c.additional_triggers.is_empty()).collect();
        let designators: Vec<bool> = cards.iter().map(|c| c.ascend || c.storied).collect();
        let engines: Vec<bool> = cards.iter().map(|c| c.start_your_engines).collect();
        let block_rules: Vec<bool> = cards.iter().map(|c| !c.block_rules.is_empty()).collect();
        let attack_rules: Vec<bool> = cards.iter().map(|c| !c.attack_rules.is_empty()).collect();
        let targeting_rules: Vec<bool> = cards.iter().map(|c| !c.targeting_rules.is_empty()).collect();
        let player_keywords: Vec<bool> = cards.iter().map(|c| !c.player_keywords.is_empty() || !c.player_protection.is_empty()).collect();
        // Like `look_back`: one it prints, one it lists to give, or any a static ability of its own could give itself.
        let mana_triggers: Vec<bool> = cards
            .iter()
            .map(|c| {
                c.abilities.iter().chain(c.delayed).any(|a| matches!(a, AbilityDef::Triggered(t) if t.mana_ability))
                    || c.statics.iter().any(|s| s.own && s.layer == Layer::Ability)
            })
            .collect();
        let state_triggers: Vec<bool> = cards
            .iter()
            .map(|c| c.abilities.iter().chain(c.delayed).any(|a| matches!(a, AbilityDef::Triggered(t) if t.state.is_some())))
            .collect();
        let ability_grants: Vec<bool> = cards.iter().map(|c| c.statics.iter().any(|s| !s.own && s.layer == Layer::Ability)).collect();
        let intrinsic = CardId(
            cards.iter().position(|c| std::ptr::eq(*c, &crate::intrinsic::INTRINSIC)).expect("the intrinsic abilities") as u16,
        );
        let face_down = CardId(
            cards.iter().position(|c| std::ptr::eq(*c, &crate::intrinsic::FACE_DOWN)).expect("the face-down values") as u16,
        );
        let cloaked = CardId(
            cards.iter().position(|c| std::ptr::eq(*c, &crate::intrinsic::CLOAKED)).expect("the cloaked values") as u16,
        );
        let no_characteristics = CardId(
            cards.iter().position(|c| std::ptr::eq(*c, &crate::intrinsic::NO_CHARACTERISTICS)).expect("no characteristics") as u16,
        );
        let undercity = cards.iter().position(|c| c.name == "Undercity" && !c.dungeon.is_empty()).map(|i| CardId(i as u16));
        let mut card_names: Vec<&'static str> = cards.iter().filter(|c| c.oracle || !c.dungeon.is_empty()).map(|c| c.name).collect();
        card_names.sort_unstable();
        let card_named = cards.iter().map(|c| card_names.binary_search(&c.name).is_ok()).collect();
        // A fused split spell's targets are its halves' together (CR 702.102b), one list for the table's life.
        let fused_targets = cards
            .iter()
            .map(|c| match c.split {
                Some((left, right)) if c.fuse => {
                    let both: Vec<TargetSpec> = left.spell_targets().iter().chain(right.spell_targets()).copied().collect();
                    Some(&*Vec::leak(both))
                }
                _ => None,
            })
            .collect();
        let mut table = CardTable {
            card_named,
            cards,
            anywhere,
            statics,
            control_statics,
            look_back,
            zone_words,
            zone_word_cards,
            ability_statics,
            graveyard_statics,
            exile_statics,
            play_rules,
            any_play_rules,
            replaces,
            any_replaces,
            any_cast_limit,
            block_rules,
            attack_rules,
            targeting_rules,
            player_keywords,
            mana_triggers,
            fused_targets,
            state_triggers,
            ability_grants,
            cost_changes,
            pay_with_grants,
            activation_rules,
            trigger_rules,
            designators,
            engines,
            intrinsic,
            face_down,
            cloaked,
            no_characteristics,
            undercity,
            backs: Vec::new(),
            halves: Vec::new(),
            fronts: Vec::new(),
            printed: Vec::new(),
        };
        let count = table.cards.len();
        table.printed = (0..count).map(|_| [std::sync::OnceLock::new(), std::sync::OnceLock::new()]).collect();
        table.backs = vec![None; count];
        table.halves = vec![None; count];
        table.fronts = vec![None; count];
        for front in 0..count {
            if let Some(back) = table.cards[front].back {
                let back = table.find(back.name).expect("a back face is in the table");
                table.backs[front] = Some(back);
                table.fronts[back.0 as usize] = Some(CardId(front as u16));
            }
            // A Room's doors (CR 709.5): the abilities and play rules of an unlocked one are the permanent's. What a door's
            // text does beyond those is read off the card, not off the door, so a door that has any is refused until it
            // follows the designations (`CardDef::door_unsupported`): never quietly absent.
            let card = table.cards[front];
            if card.is_room() {
                let (left, right) = card.split.expect("a Room is a split card");
                for half in [left, right] {
                    if let Some(what) = half.door_unsupported() {
                        panic!("{}: a door with {what} is not supported yet (CR 709.5)", half.name);
                    }
                }
                // The Room is looked at for play rules, which `actions::each_play_rule` takes from its unlocked doors.
                table.play_rules[front] |= !left.play_rules.is_empty() || !right.play_rules.is_empty();
                // And for the triggered abilities its doors print, which `layers::doors` gives it while they are unlocked:
                // those that look back (CR 603.10a), state triggers (CR 603.8) and triggered mana abilities (CR 605.1b).
                let triggered = |f: fn(&TriggeredDef) -> bool| {
                    left.abilities.iter().chain(right.abilities.iter()).any(|a| matches!(a, AbilityDef::Triggered(t) if f(t)))
                };
                table.look_back[front] |= triggered(TriggeredDef::may_look_back);
                table.state_triggers[front] |= triggered(|t| t.state.is_some());
                table.mana_triggers[front] |= triggered(|t| t.mana_ability);
                let ids = (table.find(left.name), table.find(right.name));
                table.halves[front] = Some((ids.0.expect("a door is in the card table"), ids.1.expect("a door is in the card table")));
            }
        }
        table
    }

    pub fn get(&self, id: CardId) -> &'static CardDef {
        self.cards[id.0 as usize]
    }

    /// The copiable values a permanent put onto the battlefield face down `as` has (CR 708.2).
    pub fn face_down_values(&self, how: FaceDown) -> CardId {
        match how {
            FaceDown::Manifested => self.face_down,
            FaceDown::Cloaked => self.cloaked,
        }
    }

    /// Whether `ability` is a face-down permanent's own "turn this permanent face up" (CR 701.40b, 701.58b): a special
    /// action that costs its card's mana cost.
    pub fn turns_face_up(&self, ability: AbilityRef) -> bool {
        let def = self.get(ability.card);
        def.face_down
            && ability.index == crate::intrinsic::TURN_FACE_UP
            && def.abilities.get(ability.index as usize).is_some_and(|a| a.text() == crate::intrinsic::TURN_UP_TEXT)
    }

    /// The definition an object currently has: a face-down permanent's face-down values, a copy's, else the face showing
    /// (CR 707.2, 708.2a, 712.3). The printed card of a transformed permanent is its front.
    pub fn current(&self, obj: &Obj) -> CardId {
        obj.copied_card().unwrap_or_else(|| self.face(obj.card, obj.transformed))
    }

    /// The face of a card that shows when it is transformed (CR 712.3), or the card itself when it has no back.
    pub fn face(&self, id: CardId, transformed: bool) -> CardId {
        if transformed { self.backs[id.0 as usize].unwrap_or(id) } else { id }
    }

    /// The first card of a name, in the order the registry gave (a card's own definition before the Aura face it has while
    /// bestowed, which shares its name).
    pub fn find(&self, name: &str) -> Option<CardId> {
        let first = self.cards.partition_point(|card| card.name < name);
        (self.cards.get(first).is_some_and(|card| card.name == name).then_some(first))
            .or_else(|| self.cards.iter().position(|card| card.name.eq_ignore_ascii_case(name)))
            .map(|i| CardId(i as u16))
    }

    /// What `ability`'s trigger condition says that its event can't show (`read_zone_words`).
    pub fn zone_words(&self, ability: AbilityRef) -> ZoneWords {
        if !self.zone_word_cards.get(ability.card.0 as usize).copied().unwrap_or(false) {
            return ZoneWords::default();
        }
        self.zone_words.binary_search_by_key(&ability, |&(a, _)| a).map(|at| self.zone_words[at].1).unwrap_or_default()
    }

    /// Whether the triggered ability `ability`, triggering on `event`, looks back in time (CR 603.10, `looks_back`).
    #[inline]
    pub fn looks_back(&self, ability: AbilityRef, q: &dyn Query, event: &Event) -> bool {
        match event {
            Event::ZoneChange { .. } => looks_back(self.zone_words(ability), q, event),
            _ => looks_back(ZoneWords::default(), q, event),
        }
    }

    pub fn ability(&self, ability: AbilityRef) -> &'static AbilityDef {
        if ability.is_madness() {
            return &MADNESS_ABILITY;
        }
        if ability.is_rebound() {
            return &REBOUND_ABILITY;
        }
        if ability.is_miracle() {
            return &MIRACLE_ABILITY;
        }
        let card = self.get(ability.card);
        if ability.is_delayed() {
            &card.delayed[(ability.index - AbilityRef::DELAYED) as usize]
        } else {
            &card.abilities[ability.index as usize]
        }
    }

    /// The id a definition has in this table: for token definitions and other cards' own references.
    /// The target specs a spell cast as `def` has (`CardDef::spell_targets`): a split card with fuse is cast as itself only
    /// fused, and then has both halves' (CR 702.102b).
    pub fn spell_targets_of(&self, def: &'static CardDef) -> &'static [TargetSpec] {
        if def.fuse && def.split.is_some() {
            return self.fused_targets[self.id_of(def).0 as usize].unwrap_or(&[]);
        }
        def.spell_targets()
    }

    pub fn id_of(&self, def: &'static CardDef) -> CardId {
        CardId(self.cards.iter().position(|c| std::ptr::eq(*c, def)).expect("a definition in the table") as u16)
    }
}

/// Madness's linked triggered ability (CR 702.35a), the core's own: "when this card is exiled this way, its owner may
/// cast it by paying [its madness cost] rather than its mana cost; if they don't, they put it into their graveyard."
/// The card is the ability's source, in exile where the discard put it, and its cost is the card's `madness`.
pub static MADNESS_ABILITY: AbilityDef = AbilityDef::Triggered(TriggeredDef {
    text: "Madness: cast it for its madness cost or put it into your graveyard.",
    effect: madness_resolves,
    ..TriggeredDef::BLANK
});

fn madness_resolves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    let in_exile = |g: &dyn Game| crate::game::is_live(g, card) && g.obj(card).zone.is(ZoneKind::Exile);
    // Nothing to do for a card something moved out of exile while the ability waited (CR 400.7).
    if !in_exile(g) {
        return Ok(());
    }
    let cost = g.card(g.obj(card).card).madness;
    // Nobody is asked about a card that could not be cast at all (a land, no legal target): it is simply not cast.
    if let Some(cost) = cost
        && g.could_cast_during_resolution(card)
        && matches!(g.ask(item.controller, RequestKind::May { source: card, purpose: Purpose::CastForMadness })?, Answer::Yes)
    {
        g.cast_during_resolution(card, Some(cost))?;
    }
    // "If that player doesn't, they put this card into their graveyard": still in exile means it was not cast.
    if in_exile(g) {
        let owner = g.obj(card).owner;
        g.move_to(card, Zone::graveyard(owner));
    }
    Ok(())
}

/// Miracle's linked triggered ability (CR 702.94a), the core's own: "you may cast it by paying [its miracle cost] rather
/// than its mana cost." The card is the ability's source, still in the hand it was revealed from; nobody is asked about
/// a card that could not be cast at all, and a "no", or a cast given up, leaves it where it is. The cost is that of the
/// miracle ability the card was revealed with (`Miracle::of_data`, CR 607.5): the card's own, or the one a permanent
/// gave it, which still pays though that permanent has left since (CR 113.7a).
pub static MIRACLE_ABILITY: AbilityDef = AbilityDef::Triggered(TriggeredDef {
    text: "Miracle: cast it by paying its miracle cost.",
    effect: miracle_resolves,
    ..TriggeredDef::BLANK
});

fn miracle_resolves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    let Some(miracle) = Miracle::of_data(g, card, item.data) else { return Ok(()) };
    let named = miracle.of(g.obj(card).card);
    // Each face that could be cast, with its miracle cost and what of its reduction is left for X; one whose cost can't be
    // paid (CR 118.6) is not offered.
    let ways: Vec<(CastWay, (ManaCost, u8))> = g
        .cast_ways_during_resolution(card)
        .into_iter()
        .filter_map(|way| miracle_cost_and_x_less(g, named, way.face).map(|cost| (way, cost)))
        .collect();
    if ways.is_empty() || !matches!(g.ask(item.controller, RequestKind::May { source: card, purpose: Purpose::CastForMiracleCost(named) })?, Answer::Yes) {
        return Ok(());
    }
    // Which face is cast is chosen as it is cast (CR 709.3, 715.3, 712.8a), asked as the core asks it.
    let (way, (cost, less)) = if ways.len() == 1 {
        ways[0]
    } else {
        let ask = RequestKind::ChooseModes {
            source: card,
            modes: ways.iter().map(|(w, _)| if w.way == WAY_FUSED { Label::Fused(w.face) } else { Label::Face(w.face) }).collect(),
            eligible: (0..ways.len() as u8).collect(),
            count: 1,
            at_most: 0,
            affordable: Vec::new(),
            additional_cost: false,
            repeat: false,
        };
        match g.ask(item.controller, ask)? {
            Answer::Modes(chosen) => ways[chosen.first().copied().unwrap_or(0) as usize % ways.len()],
            _ => ways[0],
        }
    };
    g.cast_during_resolution_as(card, CastTerms { instead: Some(cost), less, way: Some(way.way), ..CastTerms::default() })?;
    Ok(())
}

/// Rebound's delayed triggered ability (CR 702.88a), the core's own: "at the beginning of your next upkeep, you may
/// cast this card from exile without paying its mana cost." The card is the ability's source, in exile where its
/// resolution put it. A card that could not be cast (no legal target), a "may" answered no and a cast given up all leave
/// it in exile, and it is not offered again.
pub static REBOUND_ABILITY: AbilityDef = AbilityDef::Triggered(TriggeredDef {
    text: "Rebound: you may cast this card from exile without paying its mana cost.",
    effect: rebound_resolves,
    ..TriggeredDef::BLANK
});

fn rebound_resolves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    // Nothing to do for a card something moved out of exile while the ability waited (CR 603.7c, 400.7).
    if !crate::game::is_live(g, card) || !g.obj(card).zone.is(ZoneKind::Exile) {
        return Ok(());
    }
    if g.could_cast_during_resolution(card)
        && matches!(g.ask(item.controller, RequestKind::May { source: card, purpose: Purpose::CastFree })?, Answer::Yes)
    {
        g.cast_during_resolution(card, Some(ManaCost::ZERO))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{read_zone_words as zone_words, ZoneWords};

    /// The words read from a zone-change trigger condition (`CardTable::new`): only its condition, a list's commas included,
    /// is read, and "from anywhere other than ..." is about leaving the zones it doesn't name.
    #[test]
    fn zone_words_read_the_trigger_condition_only() {
        let w = |from_anywhere, leaves_graveyard| ZoneWords { from_anywhere, leaves_graveyard };
        assert_eq!(zone_words("Whenever another card is put into a graveyard from anywhere, exile that card."), w(true, false));
        assert_eq!(zone_words("Whenever an artifact, creature, or enchantment card is put into a graveyard from anywhere, draw a card."), w(true, false));
        assert_eq!(zone_words("Whenever one or more cards leave your graveyard, put a +1/+1 counter on target creature you control."), w(false, true));
        assert_eq!(zone_words("When this creature enters, choose target permanent card in your graveyard that was put there from anywhere this turn."), w(false, false));
        assert_eq!(zone_words("When this creature enters from anywhere other than a graveyard or exile, roll a 20-sided die."), w(false, false));
        assert_eq!(zone_words("When this creature leaves the battlefield, return the exiled card to its owner's graveyard."), w(false, false));
    }
}
