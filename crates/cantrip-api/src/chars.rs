//! Computed characteristics (CR 613): what an object is once every continuous effect has been applied.

use cantrip_state::*;


/// One object's characteristics as the layer system computed them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chars {
    /// The card whose name and printed values it has: its own, or the one it copies.
    pub card: CardId,
    /// Its name: the card's, or the one a copy effect says it has (CR 707.9b).
    pub name: &'static str,
    pub types: TypeSet,
    pub supertypes: Supertypes,
    pub subtypes: SubtypeSet,
    pub colors: ColorSet,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub keywords: Keywords,
    pub protection: ColorSet,
    /// Protection from colorless (CR 702.16a).
    pub protection_colorless: bool,
    /// Protection from players (CR 702.16a): a bit per seat, every seat a game deals (`MAX_SEATS`).
    pub protection_players: u8,
    /// Protection from objects of these card types (CR 702.16a): "protection from creatures", "protection from instants".
    pub protection_types: TypeSet,
    /// It is a spell on the stack (CR 112.1), not a card or a permanent: what "protection from spells" is about.
    pub spell: bool,
    /// Protection from spells that are one or more colors (CR 702.16a): Emrakul, the Aeons Torn.
    pub protection_colored_spells: bool,
    /// Protection from everything (CR 702.16e): from every source, whatever it is.
    pub protection_everything: bool,
    /// Its "hexproof from [quality]" abilities (CR 702.11d). Each is a hexproof ability (702.11e): what takes hexproof away
    /// takes them, and what lets a player target as though there were no hexproof lets them target past these too.
    pub hexproof_from: HexproofFrom,
    /// Attacks each combat if able (CR 508.1d).
    pub must_attack: bool,
    pub abilities: [Option<AbilityRef>; MAX_ABILITIES],
    /// It has no mana cost (CR 118.6, 707.9b): a copy that says so, whatever the card it copies prints. Its mana
    /// value is 0.
    pub no_cost: bool,
    /// A continuous effect set its name (CR 612.8): it has `name` alone, and none of the names it had (a split card's halves,
    /// a Room's doors).
    pub name_set: bool,
    /// It lost all its abilities in layer 6 (CR 613.1f): its own static abilities apply no more, from the point the loss
    /// applied, and none of the rules its card prints (`Query::lost_all_abilities`). Abilities granted after keep it.
    pub lost_all: bool,
    /// Another's effect set its land subtypes (CR 305.7): it lost the abilities its rules text and copy effects give it,
    /// from the point the effect applied in layer 4 (`set_land_subtypes`).
    pub text_lost: bool,
}

/// The words `id`'s text has now (CR 612): the ones its copiable values changed (a token made by changed text), then
/// each text-changing effect on it in timestamp order. Printed, for nearly every object. An object that has ceased to be
/// itself - changed zones or ceased to exist - has the words it last had (CR 400.7, 113.7a, 608.2h): a fetch land
/// sacrificed to its own ability searches for the land types its text named, and the delayed ability of a High Tide whose
/// "Island" was changed to "Swamp" watches Swamps once High Tide is in the graveyard.
pub fn words(state: &GameState, id: ObjectId) -> Words {
    if let Some(last) = state.last_known(id).or_else(|| state.phased_last_known(id)) {
        return last.words;
    }
    current_words(state, id)
}

/// Current words for projected characteristics, separate from a phased-out source's expected-zone information.
pub fn current_words(state: &GameState, id: ObjectId) -> Words {
    let obj = state.obj(id);
    let mut words = if obj.face_down_values().is_some() { Words::NONE } else { obj.copy.as_ref().map_or(Words::NONE, |c| c.words) };
    for e in &state.effects {
        if let EffectKind::ChangeText { from, to } = e.kind
            && e.affected == id
        {
            words = words.changed(from, to);
        }
    }
    words
}

impl Chars {
    /// The complete characteristics `last` kept (CR 400.7, 608.2h): expected-zone information, including a spell as it
    /// became cast (601.2i) and a permanent just before phasing out (702.26b).
    pub fn as_known(mut self, last: &LastKnown) -> Chars {
        self.card = last.card;
        self.name = last.name;
        self.name_set = last.name_set;
        self.supertypes = last.supertypes;
        self.no_cost = last.no_cost;
        self.types = last.types;
        self.subtypes = last.subtypes;
        self.colors = last.colors;
        self.power = last.power;
        self.toughness = last.toughness;
        self.keywords = last.keywords;
        self.protection = last.protection;
        self.protection_colorless = last.protection_colorless;
        self.protection_players = last.protection_players;
        self.protection_types = last.protection_types;
        self.spell = last.spell;
        self.protection_colored_spells = last.protection_colored_spells;
        self.protection_everything = last.protection_everything;
        self.hexproof_from = last.hexproof_from;
        self.must_attack = last.must_attack;
        self.abilities = last.abilities;
        self
    }

    pub fn is(&self, t: CardType) -> bool {
        self.types.has(t)
    }

    pub fn has(&self, k: Keyword) -> bool {
        self.keywords.has(k)
    }

    /// Whether its protection covers a source with these characteristics (CR 702.16a): a color it is protected
    /// from, or colorless.
    pub fn protected_from(&self, source: &Chars) -> bool {
        self.protection_everything
            || self.protection.intersects(source.colors)
            || (self.protection_colorless && source.colors.is_empty())
            || self.protection_types.0 & source.types.0 != 0
            || (self.protection_colored_spells && source.spell && !source.colors.is_empty())
    }

    /// Whether its protection covers a source with these characteristics controlled by `controller` (CR 702.16a):
    /// its colors, colorless, or the player controlling it.
    pub fn protected_from_by(&self, source: &Chars, controller: PlayerId) -> bool {
        self.protected_from(source) || self.protection_players & (1 << controller.0) != 0
    }

    /// "Loses all abilities" (CR 613.1f): every ability it has now goes, keywords, protections and "hexproof from"
    /// included. What is granted after it, in timestamp order, it keeps (CR 613.7).
    pub fn lose_all_abilities(&mut self) {
        self.clear_abilities();
        self.lost_all = true;
    }

    fn clear_abilities(&mut self) {
        self.keywords = Keywords::NONE;
        self.protection = ColorSet::NONE;
        self.protection_colorless = false;
        self.protection_players = 0;
        self.protection_types = TypeSet::NONE;
        self.protection_colored_spells = false;
        self.protection_everything = false;
        self.hexproof_from = HexproofFrom::NONE;
        self.must_attack = false;
        self.abilities = [None; MAX_ABILITIES];
    }

    /// "[This land] is a [basic land type]" (CR 305.7, layer 4): an effect that sets a land's subtype to basic land
    /// types. It has `types`' land types and no other (its other subtypes, an animated land's creature types, stay), and it
    /// loses the abilities its rules text and copy effects give it - every ability it has in layer 4, before any effect
    /// grants one in layer 6. The mana ability of each basic land type follows from the type in layer 6 (CR 305.6), and
    /// abilities other effects grant come after. Its card types and supertypes are not changed. For a static ability made
    /// with `sets_land_subtypes`, whose `apply` this is: the layers read that flag to know which lands lost their rules
    /// text, and so which of their static abilities, replacement effects and other printed rules no longer apply.
    /// Called from any other static ability, the land would lose the abilities among its abilities but keep those rules.
    /// Nothing for an object that is not a land: land types belong to lands (CR 205.3d).
    pub fn set_land_subtypes(&mut self, types: SubtypeSet) {
        if !self.is(CardType::Land) {
            return;
        }
        self.subtypes = self.subtypes.minus(SubtypeSet::LAND_TYPES).union(types);
        self.clear_abilities();
        self.text_lost = true;
    }

    /// The most creatures that may block it (CR 509.1b): one for "can't be blocked by more than one creature", otherwise
    /// any number (`usize::MAX`: any count of creatures there can be). With a minimum above it (menace as well), it can't be
    /// blocked at all.
    pub fn maximum_blockers(&self) -> usize {
        if self.has(Keyword::CantBeBlockedByMoreThanOne) { 1 } else { usize::MAX }
    }

    /// The fewest creatures that may block it (CR 509.1b): menace's two, "except by three or more creatures"'s three,
    /// otherwise one. A creature blocked by fewer, but at least one, is not blocked legally.
    pub fn minimum_blockers(&self) -> usize {
        if self.has(Keyword::CantBeBlockedExceptByThreeOrMore) {
            3
        } else if self.has(Keyword::Menace) {
            2
        } else {
            1
        }
    }

    pub fn has_subtype(&self, s: Subtype) -> bool {
        self.subtypes.has(s)
    }

    pub fn abilities(&self) -> impl Iterator<Item = AbilityRef> + '_ {
        self.abilities.iter().flatten().copied()
    }

    pub fn ability_count(&self) -> usize {
        self.abilities.iter().flatten().count()
    }
}
