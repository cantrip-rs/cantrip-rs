//! The public surface card code is written against (docs/engine-design.md, "Layers: core, shared mechanics,
//! cards"). Card crates and `cantrip-mechanics` depend on this crate alone, so the compiler keeps them out of the
//! rules' internals: they read the state, and change it only through [`Game`].

pub use cantrip_state::{
    AbilityRef, Action, AssistStep, ActionKind, ActionOption, Answer, Attacker, Block, CardId, CardType, Color, ColorSet, Copiable, HexproofFrom,
    CounterKind, DayNight, Duration, EffectKind, Gift, GameState, Keyword, Keywords, Label, Mana, ManaCost, ManaFrom, ManaLasts, ManaPool, ManaTally, Marks, MiracleOf, NameKind, Obj, ObjectId,
    ColorUse, WordUse, Outcome, PlayerId, Purpose, Question, Request, RequestKind, StackItem, StackKind, Step, Subtype, SubtypeSet, Supertypes,
    Target, TypeSet, Word, Words, Zone, ZoneKind, ZoneKinds, ZoneReason, DelayedWhen, ChosenOption, Chosen, ExileLink, Linked, Acquired, AcquiredBy, Expiry, ExileInstead, PlayCondition, PlayGrant, PlayPermission, LastKnown, ManaPayment, ManaRestriction, AbilitiesOf, ManaValueSpells, SubtypeSpells, MANA_GIVES_HASTE, MANA_MAKES_UNCOUNTERABLE, Destination, DestinationKind, Count, Selection, Part, Parts, DamageDivision, DamageAbout, AttackAim, AttackCost, AttackIssue, AttackOption, AttackRules, AttackVerdict, RequiredAttack, BlockIssue, BlockRequirements, BlockVerdict, RequiredBlock, CounterPool, SearchDestination, WAY_ADVENTURE, WAY_BACK_FACE, WAY_OMEN, Grant, WAY_BESTOW, WAY_OVERLOAD, WAY_SPLIT_LEFT, WAY_SPLIT_RIGHT, WAY_FUSED, WAY_TRANSFORMED, WAY_PROTOTYPE,
};

pub use cantrip_state::{count_u32, ContinuousEffect, Shield, DOOR_LEFT, DOOR_RIGHT, PAY_CONVOKE, PAY_DELVE, PAY_IMPROVISE, PAY_WATERBEND, ANY_NUMBER_OF_TIMES, PLAYER_MANA_ABILITY, MAX_ABILITIES, MAX_SEATS};

mod ability;
mod card;
mod chars;
mod event;
mod game;
pub mod history;
pub mod intrinsic;
pub mod select;
mod replace;

pub use ability::*;
pub use card::*;
pub use chars::*;
pub use event::*;
pub use game::*;
pub use history::*;
pub use replace::*;
pub use select::{Bits, Choice, SelectFn, Verdict};
