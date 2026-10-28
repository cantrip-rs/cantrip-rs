//! Pieces of static abilities (CR 604, 613) several printed cards use exactly as written.

use cantrip_api::*;

/// "Creatures you control" (CR 613.1f, layer 6): every creature under the source's controller's control, the source itself
/// among them ("creatures you control have ..." is not "other"). Judged on what the earlier layers made the object, so a
/// permanent that became a creature counts, and by control, not ownership. Nadu, Winged Wisdom and Samut, Hazoret's Champion
/// use it as `StaticDef::affects`.
pub fn your_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    so_far.is(CardType::Creature) && s.obj(affected).controller == s.obj(source).controller
}

/// "Creatures you control with +1/+1 counters on them" (CR 613.1f).
/// Agatha's Soul Cauldron and Vigorbloom Vanguard use it.
pub fn your_creatures_with_plus_one_counters(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_creatures(q, source, affected, so_far) && q.state().obj(affected).counters(CounterKind::PlusOne) > 0
}

/// "Artifact creatures you control" (CR 613.1): Tempered Steel and Vraska, Soul of Stone use it;
/// Krang, Utrom Warlord excludes itself from the same group.
pub fn your_artifact_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Artifact) && your_creatures(q, source, affected, so_far)
}

/// "Other creatures you control" (CR 613.1f, layer 6): `your_creatures` without the source itself, judged on what the earlier
/// layers made the object. Hexing Squelcher, Ulvenwald Oddity and Suki, Courageous Rescuer use it as `StaticDef::affects`.
pub fn other_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && your_creatures(q, source, affected, so_far)
}

/// "Creatures" / "all creatures" (CR 613.1f): every creature on the battlefield, whoever controls it, judged on what the
/// earlier layers made the object. Intruder Alarm and Lavaleaper ("All creatures have haste") use it as
/// `StaticDef::affects`.
pub fn all_creatures(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature)
}

/// "Planeswalkers you control" (CR 613.1f, layer 6): every planeswalker under the source's controller's control, judged on what
/// the earlier layers made it, and by control, not ownership. Avatar of Burgeoning Echoes and Way of the Pyromancer use it as
/// `StaticDef::affects`.
pub fn your_planeswalkers(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    so_far.is(CardType::Planeswalker) && s.obj(affected).controller == s.obj(source).controller
}

/// "Lands you control" (CR 613.1f, layer 6): every land under the source's controller's control, judged on what the earlier
/// layers made it, and by control, not ownership. The emblem of Nissa, Who Shakes the World and Resonating Lute use it as
/// `StaticDef::affects`.
pub fn your_lands(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    so_far.is(CardType::Land) && s.obj(affected).controller == s.obj(source).controller
}

/// "Creature tokens you control" (CR 111.1, 613.1f): a creature under the source's controller's control, as the earlier
/// layers made it, that is a token. Teysa Karlov, Caretaker's Talent (at level 3) and Gideon's Memorial use it as
/// `StaticDef::affects`.
pub fn your_creature_tokens(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().obj(affected).token && your_creatures(q, source, affected, so_far)
}

/// `your_spells_of_types::<ANY_SPELL>`: every spell, whatever its card types (Hexing Squelcher).
pub const ANY_SPELL: u16 = 0;

/// "[Creature / Instant and sorcery / No qualifier] spells you control" (CR 113.6, 601.2i, layer 6) as the `StaticDef::affects` of a
/// static ability declared `.everywhere()`: a spell on the stack (an ability on the stack has no card types) controlled by the
/// source's controller that has one of the card types in `TYPES`, a bitset of `CardType as u16` positions (`TypeSet.0`), judged on
/// what the earlier layers made it; `TYPES` of 0 is every spell. Frenzied Baloth (creature spells), Hexing Squelcher (spells) and
/// Sphinx of the Final Word and Samut, Tyrant of Naktamun (instant and sorcery spells) print it before "can't be countered".
pub fn your_spells_of_types<const TYPES: u16>(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let o = q.state().obj(affected);
    let of_a_type = if TYPES == 0 { so_far.types != TypeSet::NONE } else { so_far.types.0 & TYPES != 0 };
    o.zone.kind() == ZoneKind::Stack && of_a_type && o.controller == q.state().obj(source).controller
}

/// A static ability that affects only the object that has it (CR 604.1: "this creature", "this land", "this token") as its
/// `StaticDef::affects`. Anje's Ravager, Emrakul, the Aeons Torn, Multiversal Passage, Roaming Throne, Territorial Kavu and
/// the Construct token use it.
pub fn itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}

/// "Delirium - This creature gets/has ... as long as there are four or more card types among cards in your graveyard"
/// (CR 207.2c, 604.1, 611.3a) as a static ability's `StaticDef::affects`: only the source itself, and only while four or more
/// card types are among the cards in its controller's graveyard, each judged as the layers make it (the types are asked
/// again whenever characteristics are computed). Dragon's Rage Channeler and Wildfire Wickerfolk use it; a spell or an
/// ability asked at a `Query` uses `cantrip_mechanics::delirium`.
pub fn itself_with_delirium(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && delirium(q, source)
}

/// "During your turn, this creature has [keyword]" (CR 604.1, 613.1f) as a static ability's `StaticDef::affects`: only the
/// source itself, and only while its controller is the active player (CR 102.1), asked again whenever characteristics are
/// computed. Razorkin Needlehead, Skilled Battlecarver and Thrun, Breaker of Silence use it.
pub fn itself_during_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().turn.active == q.state().obj(source).controller
}

/// Threshold's "as long as there are seven or more cards in your graveyard" (CR 207.2c,
/// 604.1), affecting only itself. Decaying Soil and Void Extrapolator print it.
pub fn itself_with_threshold(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    let state = q.state();
    source == affected && state.zone(Zone::graveyard(state.obj(source).controller)).len() >= 7
}

/// "... has haste" (CR 702.10, layer 6) as a static ability's `StaticDef::apply`. Anger, Samut, Lavaspur Boots, Skateboard
/// and dash's haste print it.
pub fn have_haste(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Haste);
}

/// "... is a [subtype] in addition to its other types" (layer 4) as a static ability's `StaticDef::apply`, `SUBTYPE` a
/// `Subtype` as `u16` (`lands::sub`). The subtype is a word of the source's text (CR 612.2). Astrologian's Planisphere
/// (Wizard), Dark Knight's Greatsword (Knight), Samurai's Katana (Samurai) and Laughing Jasper Flint (Mercenary) print it.
pub fn also_a<const SUBTYPE: u16>(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    if let Some(word) = subtype_word(view, source, SUBTYPE) {
        chars.subtypes = chars.subtypes.with(word);
    }
}

/// The subtype a `Subtype as u16` number names, as `source`'s text has that word now (CR 612.2); `None` for a number that
/// names no subtype. `also_a`, `your_of_subtype` and `other_of_subtype_you_control` read their `SUBTYPE` through it.
fn subtype_word(view: &dyn LayerQuery, source: ObjectId, n: u16) -> Option<Subtype> {
    Subtype::ALL.into_iter().find(|&s| s as u16 == n).map(|printed| view.words(source).subtype(printed))
}

/// "[Subtype]s you control" (CR 613.1f, layer 6) as `StaticDef::affects`, `SUBTYPE` a `Subtype` as `u16` (`lands::sub`): every
/// permanent under the source's controller's control that is that creature type as the earlier layers made it, the source
/// itself among them, the type read through the words of the source's text (CR 612.2). Corpses of the Lost (Skeletons) and
/// Saheeli, Jewel of Avishkar (Thopters) print it.
pub fn your_of_subtype<const SUBTYPE: u16>(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    subtype_word(q, source, SUBTYPE).is_some_and(|word| so_far.has_subtype(word))
        && q.state().obj(affected).controller == q.state().obj(source).controller
}

/// "Other [subtype]s you control" (CR 613.1f, layer 6): `your_of_subtype` without the source itself. Coppercoat Vanguard
/// (Humans), Lyra Dawnbringer (Angels) and Magda, Brazen Outlaw (Dwarves) print it.
pub fn other_of_subtype_you_control<const SUBTYPE: u16>(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && your_of_subtype::<SUBTYPE>(q, source, affected, so_far)
}

/// "As long as you control another [subtype], this creature has ..." (CR 604.1, 613.1f) as `StaticDef::affects`, `SUBTYPE` a
/// `Subtype` as `u16` (`lands::sub`): only the source itself, and only while its controller controls another permanent with
/// that creature type, as the earlier layers made it, the type read through the words of the source's text (CR 612.2). Its own
/// type does not count, and an opponent's permanent is not one "you control". Dain's Company (lifelink, Dwarf) and Bolg's Company
/// (haste, Goblin) print it.
pub fn itself_while_you_control_another<const SUBTYPE: u16>(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    if affected != source {
        return false;
    }
    let Some(word) = subtype_word(view, source, SUBTYPE) else { return false };
    let state = view.state();
    let you = state.obj(source).controller;
    state.battlefield().iter().any(|&id| id != source && state.obj(id).controller == you && view.chars(id).has_subtype(word))
}

/// "... can't be countered" (CR 101.2, layer 6) as the `StaticDef::apply` of a static ability that covers spells on the
/// stack (declared `everywhere`). Hexing Squelcher ("Spells you control") and Frenzied Baloth ("Creature spells you
/// control") print it.
pub fn cant_be_countered(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBeCountered);
}

/// "This permanent doesn't untap during your untap step" (CR 502.3) as the `StaticDef::apply`, in `Layer::Ability`, of a
/// permanent's own static ability that applies to itself only some of the time (Bombur, Gentle Dreamer's "unless you have an
/// enduring story"): an ability of the permanent, so it goes with its abilities (CR 613.1f). Another permanent's "doesn't
/// untap" (Intruder Alarm, Infinite Coursework) is no ability of the permanents it is about and never a keyword of theirs:
/// it is `CardDef::untap_bans`.
pub fn does_not_untap(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::DoesNotUntap);
}

/// "Enchanted creature doesn't untap during its controller's untap step" (CR 502.3) as a `CardDef::untap_bans` entry: a rule of
/// the Aura's (or the Equipment's), not an ability of the permanent it is attached to (CR 113.1a), so the permanent losing all
/// its abilities leaves it in force (CR 613.1f). Infinite Coursework and Enchanted River's Grasp print it.
pub fn attached_permanent_doesnt_untap(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.obj(asker.source).attached_to == Some(id)
}

/// "... has flying" (CR 702.9, layer 6) as a static ability's `StaticDef::apply`. Dragon's Rage Channeler's delirium and Iron
/// Man Armor's equipped creature print it.
pub fn have_flying(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Flying);
}

/// "... has first strike" (CR 702.7, layer 6) as a static ability's `StaticDef::apply`. Razorkin Needlehead and Skilled
/// Battlecarver print it.
pub fn have_first_strike(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::FirstStrike);
}

/// "... has indestructible" (CR 702.12, layer 6) as a static ability's `StaticDef::apply`. Doctor Doom, Hammer of Nazahn's
/// equipped creature and Thrun, Breaker of Silence print it.
pub fn have_indestructible(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Indestructible);
}

/// "... has flying and haste" (CR 702.9, 702.10, layer 6). Detective's Phoenix and Whiplash Wordsmith print it.
pub fn have_flying_and_haste(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Flying).with(Keyword::Haste);
}

/// "... has hexproof" (CR 702.11, layer 6) as a static ability's `StaticDef::apply`. Kaito, Bane of Nightmares' ninja form,
/// Shardmage's Rescue's enchanted creature and Stoic Sphinx print it.
pub fn have_hexproof(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Hexproof);
}

/// "[These] have split second" (CR 702.61a, layer 6): `StaticDef::apply` for a static ability that grants it, with
/// `your_spells_of_types` as its `affects`. Samut, Tyrant of Naktamun prints it.
pub fn have_split_second(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::SplitSecond);
}

/// "... has menace" (CR 702.111, layer 6) as a static ability's `StaticDef::apply`. Carnage Interpreter, Elenda, Saint of Dusk
/// and Great Ugly-Looking Goblin's creatures with +1/+1 counters print it.
pub fn have_menace(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Menace);
}

/// "... has deathtouch" (CR 702.2, layer 6) as a static ability's `StaticDef::apply`. Bow of Nylea's attacking creatures and
/// The Swarmweaver's delirium Insects and Spiders print it.
pub fn have_deathtouch(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Deathtouch);
}

/// "... has deathtouch and lifelink" (CR 702.2, 702.15, layer 6) as a static ability's `StaticDef::apply`. Basilisk Collar's and
/// Dissection Tools' equipped creature.
pub fn have_deathtouch_and_lifelink(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Deathtouch).with(Keyword::Lifelink);
}

/// Delirium (an ability word, CR 207.2c) for a static ability's condition: four or more card types among the cards in the
/// graveyard of `source`'s controller, judged on the cards' characteristics as the layers made them. `mechanics::delirium` is
/// the same question outside the layers. `itself_with_delirium` (Dragon's Rage Channeler, Wildfire Wickerfolk) and The
/// Swarmweaver's Insects and Spiders use it.
pub fn delirium(view: &dyn LayerQuery, source: ObjectId) -> bool {
    let state = view.state();
    let you = state.obj(source).controller;
    let mut seen = TypeSet::NONE;
    for &card in state.zone(Zone::graveyard(you)) {
        for t in view.chars(card).types.iter() {
            seen = seen.with(t);
        }
    }
    seen.iter().count() >= 4
}

/// Spell mastery (an ability word, CR 207.2c) for a static ability's condition: two or more instant and/or sorcery cards in the
/// graveyard of `source`'s controller, judged on the cards' characteristics as the layers made them. `mechanics::spell_mastery`
/// is the same question outside the layers. Ghitu Lavarunner uses it; Fiery Impulse asks the other.
pub fn spell_mastery(view: &dyn LayerQuery, source: ObjectId) -> bool {
    let state = view.state();
    let you = state.obj(source).controller;
    state.zone(Zone::graveyard(you)).iter().filter(|&&c| view.chars(c).is(CardType::Instant) || view.chars(c).is(CardType::Sorcery)).count() >= 2
}

/// "... has vigilance" (CR 702.20, layer 6) as a static ability's `StaticDef::apply`. Ori, Keeper of Songs, Gideon's Memorial's
/// creature tokens and Teysa Karlov's print it.
pub fn have_vigilance(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Vigilance);
}

/// "Each artifact you control" counted for a static ability (CR 613.4c): the permanents under the controller of `source` that
/// the earlier layers made artifacts, `source` itself among them if it is one. "+1/+1 for each artifact you control"
/// (below, Adaptive Omnitool and others) and Improvised Arsenal's "+1/+0 for each artifact you control" both count them.
pub fn artifacts_you_control(view: &dyn LayerQuery, source: ObjectId) -> i16 {
    let state = view.state();
    let you = state.obj(source).controller;
    state.battlefield().iter().filter(|&&id| state.obj(id).controller == you && view.chars(id).is(CardType::Artifact)).count() as i16
}

/// "Gets +1/+1 for each artifact you control" (CR 613.4c, layer 7c): "you" is the controller of the source, the object that has
/// the ability (CR 109.5, 113.6), counted as the earlier layers left every permanent, the source itself among them if it is an
/// artifact. The Construct token (Urza's Saga, Karn, Scion of Urza), Adaptive Omnitool's equipped creature and Iron Man
/// Armor's animated Equipment print it.
pub fn plus_one_plus_one_per_artifact_you_control(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let n = artifacts_you_control(view, source);
    chars.power = chars.power.map(|v| v + n);
    chars.toughness = chars.toughness.map(|v| v + n);
}

/// "[This creature]'s power is equal to the number of creatures you control" (CR 604.3, layer 7a): a characteristic-defining
/// ability, the creature itself among those counted if it is one. Adeline, Resplendent Cathar and Esgaroth Garrison print it.
pub fn power_is_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(crate::lands::count_you_control(q, source, &|c| c.is(CardType::Creature)));
}

/// "... has trample" (CR 702.19, layer 6) as a static ability's `StaticDef::apply`. Ajani's Anguish, Keen-Eyed Curator, Orcrist
/// and Rancor print it.
pub fn have_trample(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Trample);
}

/// "... has double strike and trample" (CR 702.4, 702.19, layer 6) as a static ability's `StaticDef::apply`. Embercleave and
/// Leyline Axe print it.
pub fn have_double_strike_and_trample(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::DoubleStrike).with(Keyword::Trample);
}

/// "If this card is in your opening hand, you may begin the game with it on the battlefield" (CR 103.6a), for `CardDef::opening_hand`:
/// any player may, nothing else is required. The Leylines (Punishment and Resonance) and Leyline Axe print it.
pub const BEGIN_ON_THE_BATTLEFIELD: OpeningHandAction = OpeningHandAction { condition: in_any_opening_hand, counter: None, exile_from_hand: 0 };

fn in_any_opening_hand(_: &dyn Query, _: &Asker) -> bool {
    true
}

/// "This creature gets +1/+0 as long as any player controls a [color] permanent" (layer 7c) as a static ability's
/// `StaticDef::apply`, `COLOR` a `Color` as `u8`: no matter how many there are, and whoever controls them. The color is a
/// word of the source's text (CR 612.2), and a permanent's color is what the layers before this one made it. Knight of
/// Grace (black) and Knight of Malice (white) print it.
pub fn plus_one_zero_while_any_player_controls<const COLOR: u8>(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let color = view.words(source).color(Color::ALL[COLOR as usize]);
    let state = view.state();
    if state.battlefield().iter().any(|&id| view.chars(id).colors.has(color)) {
        chars.power = chars.power.map(|p| p + 1);
    }
}

/// The permanent an event put onto the battlefield (CR 603.6a): a zone change to the battlefield, a token's creation
/// included. What stops "[permanents] entering" from causing abilities to trigger asks it of the event, then judges the
/// permanent as it exists on the battlefield (`Query::chars` of the new object: copy effects and the other continuous effects
/// that apply to it, the rulings' March of the Machines). Torpor Orb, Doorkeeper Thrull and Hushbringer read it.
pub fn entering(event: &Event) -> Option<ObjectId> {
    match *event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) => Some(new),
        _ => None,
    }
}

/// Whether an event is a creature dying that causes this ability to trigger (CR 700.4, 603.10a): a creature put into a
/// graveyard from the battlefield, judged as it last existed there (a land that had become a creature dies as a creature).
/// Two abilities the event matches are not caused by the dying (the rulings of Hushbringer, Teysa Karlov and Drivnod): one
/// whose trigger event is the sacrifice that caused it (CR 701.21a, "whenever you sacrifice a creature"), and the dying card's
/// own ability that triggers when it is put into a graveyard from anywhere, which functions from the graveyard it reached.
/// Hushbringer ("creatures ... dying don't cause abilities to trigger"), Teysa Karlov and Drivnod, Carnage Dominus ("if a
/// creature dying causes a triggered ability ... to trigger") ask it.
pub fn creature_dying(q: &dyn Query, event: &Event, triggering: &Triggering) -> bool {
    match *event {
        Event::ZoneChange { old, new, from, to, .. } if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) => {
            q.chars(old).is(CardType::Creature) && !triggering.def.on_sacrifice && triggering.source != new
        }
        _ => false,
    }
}

/// "If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an
/// additional time" (CR 603.2d, 603.6a) as a `CardDef::additional_triggers` entry: the permanent's own enters abilities and
/// every other ability its entering triggers, whoever controls the permanent (the rulings). Elesh Norn, Mother of Machines,
/// Yarok, the Desecrated, Starfield Vocalist and Virtue of Knowledge print it.
pub fn permanent_entering_triggers_again(_: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    entering(event).is_some() && triggering.of_a_permanent_controlled_by(asker.controller)
}

/// "If a creature dying causes a triggered ability of a permanent you control to trigger, that ability triggers an additional
/// time" (CR 603.2d) as a `CardDef::additional_triggers` entry; what counts as the dying is [`creature_dying`]. Teysa Karlov
/// and Drivnod, Carnage Dominus print it.
pub fn creature_dying_triggers_again(q: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller) && creature_dying(q, event, triggering)
}

/// "If a creature attacking causes a triggered ability of a permanent you control to trigger, that ability triggers an
/// additional time" (CR 603.2d, 508.2a) as a `CardDef::additional_triggers` entry: abilities whose trigger is a creature being
/// declared as an attacker, "whenever [this] attacks" and "whenever you attack" alike, not one that triggers on something else
/// attacking does, such as becoming tapped (the rulings). Isshin, Two Heavens as One and Windcrag Siege (Mardu) print it.
pub fn creature_attacking_triggers_again(_: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    matches!(event, Event::Attacks { .. } | Event::AttackersDeclared { .. }) && triggering.of_a_permanent_controlled_by(asker.controller)
}

/// "Creatures entering don't cause abilities to trigger" (CR 603.2, 603.6a) as a `CardDef::trigger_rules` entry: an
/// event that puts a creature onto the battlefield triggers no ability, the creature's own enters abilities included and
/// whoever controls them. Replacement effects and "as [this] enters" are not triggered abilities (CR 614.1c) and still
/// apply. Torpor Orb, Tocatli Honor Guard and Hushwing Gryff print it; Hushbringer prints it with "or dying".
pub fn creatures_entering_dont_trigger(q: &dyn Query, _: &Asker, event: &Event, _: &Triggering) -> bool {
    entering(event).is_some_and(|permanent| q.chars(permanent).is(CardType::Creature))
}

/// "Players can't cast spells from graveyards or libraries" (CR 101.2, 601.3) as a `CardDef::cast_rules` entry: anyone's, from any
/// of them. A card exiled from a library and cast is cast from exile, and playing a land is not casting (CR 305.1).
/// Grafdigger's Cage and Weathered Runestone print it.
pub fn players_cant_cast_from_graveyards_or_libraries(_: &dyn Query, _: &Asker, _: PlayerId, from: ZoneKind) -> bool {
    !matches!(from, ZoneKind::Graveyard | ZoneKind::Library)
}

/// "Players can't gain life" (CR 119.7) as a `CardDef::cant_gain_life` entry: every player, its controller included, while
/// the permanent is on the battlefield. Sunspine Lynx, Giant Cindermaw, Rampaging Ferocidon, Leyline of Punishment, Everlasting
/// Torment and Havoc Festival print it.
pub fn players_cant_gain_life(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}

/// "Your opponents can't gain life" (CR 119.7, 102.2) as a `CardDef::cant_gain_life` entry: every player but its controller.
/// Knight of Dusk's Shadow, Erebos, God of the Dead, Gríma Wormtongue, Quakebringer and The Lord of Pain print it.
pub fn opponents_cant_gain_life(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player != asker.controller
}

/// "You can't lose the game" (CR 104.3) as a `CardDef::loss_rules` entry: however its controller would lose - 0 or less life, a
/// draw from an empty library, ten poison counters, an effect that says so - they don't, while the permanent is on the
/// battlefield with its abilities. A concession is no effect and still loses (CR 104.3a). Platinum Angel and Herald of
/// Eternal Dawn print it.
pub fn you_cant_lose_the_game(_: &dyn Query, asker: &Asker, player: PlayerId, _: LossReason) -> bool {
    player == asker.controller
}

/// "Your opponents can't win the game" (CR 104.2b, 102.2) as a `CardDef::win_rules` entry: an effect that says an opponent wins
/// does nothing. Platinum Angel and Herald of Eternal Dawn print it.
pub fn your_opponents_cant_win_the_game(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player != asker.controller
}

/// "Creatures you control can't have -1/-1 counters put on them" (CR 122.6, 101.2) as a `CardDef::counter_bans` entry: by damage
/// from a source with wither or infect (CR 120.3d, which is still damage dealt), by an effect, or as the creature enters with
/// them (persist's, CR 702.79a, 122.6), judged on the creature as it would have them and under the player it would be
/// controlled by. The counters already on a creature stay (the 2011-06-01 ruling). Darksteel Angel and Melira, Sylvok Outcast
/// print it.
pub fn no_minus_counters_on_your_creatures(q: &dyn Query, asker: &Asker, object: ObjectId, controller: PlayerId, kind: CounterKind) -> bool {
    kind == CounterKind::MinusOne && controller == asker.controller && q.chars(object).is(CardType::Creature)
}

/// "You have shroud" (CR 702.18a) as a `CardDef::player_keywords` entry: the permanent's controller, while it is on the
/// battlefield with its abilities. True Believer and Ivory Mask print it.
pub const YOU_HAVE_SHROUD: PlayerKeywordRule = PlayerKeywordRule { keyword: Keyword::Shroud, applies: you };

/// "You have protection from each of your opponents" (CR 702.16i, 702.16k) as a `CardDef::player_protection` entry: the
/// permanent's controller has a protection from each player who is their opponent, as control and the opponents are now, while
/// it is on the battlefield with its abilities. A keyword ability, so shared from its first card: Absolute Virtue prints it.
pub const YOU_HAVE_PROTECTION_FROM_EACH_OPPONENT: PlayerProtectionRule = PlayerProtectionRule { applies: you_from_each_opponent };

fn you_from_each_opponent(q: &dyn LayerQuery, source: ObjectId, player: PlayerId, from: PlayerId) -> bool {
    you(q, source, player) && q.state().opponents(player).any(|o| o == from)
}

/// "You have hexproof" (CR 702.11c) as a `CardDef::player_keywords` entry: the permanent's controller, while it is on the
/// battlefield with its abilities, can't be the target of spells or abilities their opponents control. Crystal Barricade,
/// Leyline of Sanctity and Witchbane Orb print it.
pub const YOU_HAVE_HEXPROOF: PlayerKeywordRule = PlayerKeywordRule { keyword: Keyword::Hexproof, applies: you };

/// "You": the source's controller, as a [`PlayerKeywordRule`] reads it.
fn you(q: &dyn LayerQuery, source: ObjectId, player: PlayerId) -> bool {
    q.state().obj(source).controller == player
}

/// Whether `n` more cards would be more than one card `player` has drawn this turn (CR 121.2b): the limit of "can't draw
/// more than one card each turn", counted from the history, so the cards drawn before the permanent arrived count
/// (Narset, Parter of Veils' ruling) and a draw a replacement effect replaced does not (CR 614.6). Spirit of the Labyrinth,
/// Narset, Parter of Veils and Leovold, Emissary of Trest print the limit.
pub fn more_than_one_card_drawn_this_turn(q: &dyn Query, player: PlayerId, n: u32) -> bool {
    crate::history::drawn_this_turn(q, player) as u32 + n > 1
}

/// "Each opponent can't draw more than one card each turn" (CR 121.2b) as a `CardDef::cant_draw` entry: every player but
/// its controller. Narset, Parter of Veils and Leovold, Emissary of Trest print it.
pub fn opponents_cant_draw_more_than_one_card_each_turn(q: &dyn Query, asker: &Asker, player: PlayerId, n: u32) -> bool {
    player != asker.controller && more_than_one_card_drawn_this_turn(q, player, n)
}

/// "Damage can't be prevented" (CR 615.12) as a `CardDef::unpreventable_damage` entry: all damage, from any source, combat
/// or not, while the permanent is on the battlefield. Sunspine Lynx, Leyline of Punishment and Everlasting Torment print it.
pub fn damage_cant_be_prevented(_: &dyn Query, _: &Asker, _: ObjectId, _: bool) -> bool {
    true
}

/// "Gets +P/+T" (CR 613.4c): the power and toughness change of an Equipment's, Aura's or permanent's static ability, as the
/// `apply` of a `Layer::PtModify` static (a negative number is a minus). Cori-Steel Cutter, Embercleave, Grafted Wargear,
/// Doc Ock's Tentacles and the other plain "+N/+M" statics print it.
pub fn plus_pt<const P: i16, const T: i16>(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v + P);
    chars.toughness = chars.toughness.map(|v| v + T);
}

/// "Creatures you control get +1/+1." (CR 613.4c, layer 7c): every creature its controller controls, itself among them once
/// it is one (an Opalescence-animated Glorious Anthem pumps itself), judged on what the earlier layers made each object.
/// Glorious Anthem, Warleader's Call and Cosmium Kiln (Clay-Fired Bricks' back face) print it.
pub const CREATURES_YOU_CONTROL_GET_PLUS_ONE: StaticDef =
    static_ability(Layer::PtModify, your_creatures, plus_pt::<1, 1>, "Creatures you control get +1/+1.");

/// "Has base power and toughness P/T" (CR 613.4b, layer 7b) as a static ability's `apply`: it sets the power and toughness
/// the counters and effects that modify them (layer 7c) then apply to. Humility and Eaten by Piranhas (1/1), Hexdrinker (4/4
/// and 6/6), Kaito's Ninja (3/4), Puppet Crafting (5/5), Wrecking Ball Arm (7/7) and a station card's striation (Wurmwall
/// Sweeper's 2/2, CR 721.2b) print it.
pub fn base_pt<const P: i16, const T: i16>(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = Some(P);
    chars.toughness = Some(T);
}

/// "Spells you cast" (CR 601.2i, 707.10): `affected` is a spell on the stack that was cast, from the zone it was cast from
/// (a copy of a spell was cast from none, CR 707.10; a copy of a card that was cast, a prepare spell's, CR 707.12, 722.3c,
/// is a spell cast), and its controller is `source`'s controller. Thrumming Stone and Ral, Crackling Wit's emblem grant
/// their abilities to such spells.
pub fn spell_you_cast(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId) -> bool {
    let s = view.state();
    let obj = s.obj(affected);
    obj.zone.is(ZoneKind::Stack)
        && obj.cast_from.is_some()
        && s.stack.iter().any(|i| i.kind == StackKind::Spell && i.object == affected && i.controller == s.obj(source).controller)
}

/// "Activated abilities of sources with the chosen name can't be activated unless they're mana abilities." (CR 602.5, 605.1a) as a
/// `CardDef::activation_rules` entry: a source is anything with the name (`library::object_has_name`), in any zone it has the
/// ability in. It binds every player, the permanent's controller too; with no name chosen it stops nothing. Petrified Hamlet (a
/// land card name) and Sorcerous Spyglass (any card name) print it.
pub fn chosen_name_activations_barred(q: &dyn Query, asker: &Asker, source: ObjectId, def: &ActivatedDef) -> bool {
    def.is_mana_ability() || !q.state().chosen_card(q.linked_to(asker)).is_some_and(|name| crate::library::object_has_name(q, source, name))
}

/// "Has all activated abilities of [these cards]" (CR 602.1, 613.1f), a layer-6 apply: each activated ability each of
/// `cards` prints, in order, the affected object's own as written, so it is the ability's source and "this [card]" means it
/// (CR 113.7, the Locus of Enlightenment ruling). A special action is no ability (CR 116.2). The same ability twice is two
/// abilities (CR 602.5c). Agatha's Soul Cauldron (the creature cards exiled with it) and Locus of Enlightenment (the exiled
/// cards used to craft it) grant them.
pub fn grant_activated_abilities_of(view: &dyn LayerQuery, cards: &[CardId], chars: &mut Chars) {
    for &card in cards {
        for (index, ability) in view.cards().get(card).abilities.iter().enumerate() {
            if !matches!(ability, AbilityDef::Activated(a) if !a.special_action) {
                continue;
            }
            if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
                chars.abilities[slot] = Some(AbilityRef { card, index: index as u8 });
            }
        }
    }
}

/// "Has [an ability]" (CR 113.6, 613.1f) for the card's delayed ability number 0: a static ability that gives the objects it
/// affects the ability the card lists first in `delayed` (the card as the source has it now: the face showing, or the card it copies).
/// Astrologian's Planisphere, Decaying Soil, Deconstruction Hammer, Footfall Crater, Hexing Squelcher, Innkeeper's Talent,
/// Kaldra Compleat, Lavaspur Boots, Nadu, Paradise Mantle, Petrified Hamlet, Quicksilver Dagger, Sheltered by Ghosts, Thorin
/// Oakenshield and Thrumming Stone grant theirs this way.
pub fn grant_delayed_ability(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let card = view.cards().current(view.state().obj(source));
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "Creatures can't attack you unless their controller pays {N} for each creature they control that's attacking you" (CR
/// 508.1d, 508.1h) as a `CardDef::attack_costs` entry: each creature attacking the source's controller costs {N} to
/// declare; one attacking a planeswalker they control or a battle they protect costs nothing (the rulings: "a creature that
/// can't attack you can still attack a planeswalker you control"). Ghostly Prison, Propaganda and Windborn Muse print it
/// with {2}; Dáin, Lord of the Iron Hills with {1}, while its controller has an enduring story.
pub fn attacking_you_costs<const N: u8>(_: &dyn Query, asker: &Asker, _: ObjectId, target: Target) -> Option<ManaCost> {
    (target == Target::Player(asker.controller)).then_some(ManaCost { generic: N, ..ManaCost::ZERO })
}

/// "Creatures can't attack you or planeswalkers you control unless their controller pays {N} for each of those creatures"
/// (CR 508.1d, 508.1h) as a `CardDef::attack_costs` entry: each creature attacking the source's controller or a planeswalker
/// they control costs {N} to declare; one attacking a battle costs nothing. Baird, Steward of Argive and Archon of
/// Absolution print it with {1}.
pub fn attacking_you_or_your_planeswalkers_costs<const N: u8>(q: &dyn Query, asker: &Asker, _: ObjectId, target: Target) -> Option<ManaCost> {
    attacks_you_or_your_planeswalker(q, asker, target).then_some(ManaCost { generic: N, ..ManaCost::ZERO })
}

/// Whether an attack on `target` is one on "you or planeswalkers you control" of an attack cost (CR 508.1d): the source's
/// controller, or a planeswalker they control; never a battle. Baird, Steward of Argive and Archon of Absolution ask it
/// with {1} (`attacking_you_or_your_planeswalkers_costs`), Norn's Annex with {W/P}.
pub fn attacks_you_or_your_planeswalker(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    match target {
        Target::Player(p) => p == asker.controller,
        Target::Object(id) => q.chars(id).is(CardType::Planeswalker) && q.controller(id) == asker.controller,
        _ => false,
    }
}

/// "Artifacts and creatures entering don't cause abilities to trigger" (CR 603.2, 603.6a) as a `CardDef::trigger_rules` entry:
/// an event that puts an artifact or a creature onto the battlefield, judged as it is there (the continuous effects applied),
/// triggers no ability, the permanent's own enters abilities included. A land that is an artifact or a creature too stops a
/// landfall ability; one that is neither does not (the rulings' Tireless Tracker). Doorkeeper Thrull and Karn, Argent
/// Defender print it.
pub fn artifacts_and_creatures_entering_dont_trigger(q: &dyn Query, _: &Asker, event: &Event, _: &Triggering) -> bool {
    entering(event).is_some_and(|permanent| {
        let chars = q.chars(permanent);
        chars.is(CardType::Artifact) || chars.is(CardType::Creature)
    })
}

/// "Nonbasic lands ...": every land without the basic supertype, judged as layers 1 to 3 left it (CR 205.4a, 613.1d).
fn nonbasic_land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Land) && !so_far.supertypes.basic
}

/// "... are Mountains" (CR 305.7, layer 4): the land's subtype is set to Mountain, the word of the source's text (CR 612).
/// It loses its other land types and the abilities of its rules text, and has "{T}: Add {R}" (CR 305.6); its name and
/// supertypes stay (the cards' rulings).
fn mountains(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.set_land_subtypes(SubtypeSet::NONE.with(view.words(source).subtype(Subtype::Mountain)));
}

/// "Nonbasic lands are Mountains." (CR 305.7, 613.1d): Blood Moon and Magus of the Moon print it. Applied in layer 4 to
/// every nonbasic land, whoever controls it, it goes on applying when its source loses its abilities in layer 6 (CR 613.6,
/// Magus of the Moon's ruling), and a land with an ability of its own that changes land types (Urborg) loses that ability
/// to it first (CR 613.8a, `sets_land_subtypes`).
pub const NONBASIC_LANDS_ARE_MOUNTAINS: StaticDef = sets_land_subtypes(nonbasic_land, mountains, "Nonbasic lands are Mountains.");

/// "... are every basic land type in addition to their other types" (CR 305.7, 205.1b, layer 4): the land keeps its land
/// types and rules text and gains Plains, Island, Swamp, Mountain and Forest, and with them each one's mana ability (CR
/// 305.6). "Every basic land type" names no basic land type, so no text-changing effect changes it (CR 612.2). Prismatic
/// Omen, Dryad of the Ilysian Grove and Leyline of the Guildpact print it, for `your_lands`.
pub fn every_basic_land_type(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    if chars.is(CardType::Land) {
        chars.subtypes = chars.subtypes.union(SubtypeSet::of(&[Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest]));
    }
}
