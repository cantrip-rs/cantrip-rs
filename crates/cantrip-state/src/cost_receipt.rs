//! Exact identities for objects used in paying costs.

/// Provenance of one separately announced component of a spell's total cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostOrigin {
    /// The locked-in combined mana cost, after cost changes and reductions.
    ManaTotal,
    CastingWay,
    PrintedAdditional,
    PermissionAdditional,
    AdditionalChoice(usize),
    Optional,
    Kicker(usize),
    Modal(usize),
    Targets,
    GrantedOptional(usize),
}

/// The distinct part within a component that an object paid. A discard index names an instruction in the component's
/// fixed-slot `Discards`; independently composed `Cost::sequence` components keep their separate `usize` component ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CostPart { Permanents, Variable, SelfPermanent, Granter, CountersAmong, Blight, GraveyardExile, HandExile, Evidence, ExileSymbols, Discard(usize), DiscardHand, Behold, BeholdExile, TapCreatures, TapSelf, ReturnSelf, ExileSelf, DiscardSelf, RemoveSourceCounters, PutSourceCounters, VariableSourceCounters, RemoveAnySourceCounter, Loyalty, ExertSelf, UnattachSelf, CraftSource, CraftMaterial, LibrarySelf, Convoke, Delve, Improvise, Waterbend, TapForMana, Choose }

/// Immutable provenance shared by a recorded cost-object event and history queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CostReceiptDetails {
    pub origin: Option<CostOrigin>,
    pub component: usize,
    pub part: CostPart,
}
