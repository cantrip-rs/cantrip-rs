//! Protection that doesn't remove some Auras (CR 702.16n): "Enchanted creature has protection from black. This effect doesn't
//! remove this Aura." (Black Ward) and "This effect doesn't remove Auras." (Spectra Ward). The Auras it names aren't put into
//! their owners' graveyards by it as a state-based action (CR 704.5m, 702.16c); another instance of protection from the same
//! quality still removes them, as normal. It is protection in every other way: an Aura can't be attached to the creature
//! (Spectra Ward's ruling), the protection covers the Aura's own damage and targeting (CR 702.16b, 702.16e), and an Equipment,
//! no Aura, still falls off (CR 702.16d). The colors are judged as the layers leave them: an Aura made the protected color in
//! layer 5 (Painter's Servant), or one whose color word is changed in layer 3 (Sleight of Mind), is still spared. Over test cards
//! shaped like Black Ward, White Ward, Cho-Manno's Blessing, Spectra Ward and Tattoo Ward.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::protection;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const WHITE: u8 = Color::White as u8;
const BLACK: u8 = Color::Black as u8;

static LAND: CardDef = CardDef { name: "Sparing Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Sparing Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature_on_the_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// An "enchant creature" Aura of `colors` with `statics`.
const fn aura(name: &'static str, colors: ColorSet, statics: &'static [StaticDef]) -> CardDef {
    CardDef {
        name,
        colors: Some(colors),
        types: TypeSet::of(&[CardType::Enchantment]),
        subtypes: SubtypeSet::of(&[Subtype::Aura]),
        enchant: Some(target(creature_on_the_battlefield)),
        statics,
        ..CardDef::BLANK
    }
}

/// White Ward: a white Aura that gives protection from white and doesn't remove itself.
static WHITE_WARD: CardDef = aura(
    "Sparing White Ward",
    ColorSet::of(Color::White),
    &[protection::enchanted_from_color_not_removing_this_aura::<WHITE>(
        "Enchanted creature has protection from white. This effect doesn't remove this Aura.",
    )],
);

/// Black Ward: a white Aura that gives protection from black and doesn't remove itself.
static BLACK_WARD: CardDef = aura(
    "Sparing Black Ward",
    ColorSet::of(Color::White),
    &[protection::enchanted_from_color_not_removing_this_aura::<BLACK>(
        "Enchanted creature has protection from black. This effect doesn't remove this Aura.",
    )],
);

/// Cho-Manno's Blessing: protection from the color chosen as it entered, and it doesn't remove itself.
static CHOSEN_WARD: CardDef = aura(
    "Sparing Chosen Ward",
    ColorSet::of(Color::White),
    &[protection::enchanted_from_chosen_color_not_removing_this_aura(
        "Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura.",
    )],
);

/// Spectra Ward's "protection from each color. This effect doesn't remove Auras.", each color a word of its text.
fn from_each_color_not_removing_auras(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let words = view.words(source);
    let colors = Color::ALL.into_iter().fold(ColorSet::NONE, |set, c| set.with(words.color(c)));
    chars.protect_sparing(colors, TypeSet::NONE, Spares::Auras);
}

static SPECTRA_WARD: CardDef = aura(
    "Sparing Spectra Ward",
    ColorSet::of(Color::White),
    &[static_ability(
        Layer::Ability,
        attached_object,
        from_each_color_not_removing_auras,
        "Enchanted creature has protection from each color. This effect doesn't remove Auras.",
    )],
);

/// Tattoo Ward's "protection from enchantments. This effect doesn't remove this Aura."
fn from_enchantments_not_removing_this_aura(_: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.protect_sparing(ColorSet::NONE, TypeSet::of(&[CardType::Enchantment]), Spares::Aura(source));
}

static TATTOO_WARD: CardDef = aura(
    "Sparing Tattoo Ward",
    ColorSet::of(Color::White),
    &[static_ability(
        Layer::Ability,
        attached_object,
        from_enchantments_not_removing_this_aura,
        "Enchanted creature has protection from enchantments. This effect doesn't remove this Aura.",
    )],
);

static WHITE_AURA: CardDef = aura("Sparing White Aura", ColorSet::of(Color::White), &[]);
static BLACK_AURA: CardDef = aura("Sparing Black Aura", ColorSet::of(Color::Black), &[]);
static BLUE_AURA: CardDef = aura("Sparing Blue Aura", ColorSet::of(Color::Blue), &[]);

/// A white Equipment: no Aura, so nothing spares it.
static WHITE_EQUIPMENT: CardDef = CardDef {
    name: "Sparing White Equipment",
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &LAND,
        &BEAR,
        &WHITE_WARD,
        &BLACK_WARD,
        &CHOSEN_WARD,
        &SPECTRA_WARD,
        &TATTOO_WARD,
        &WHITE_AURA,
        &BLACK_AURA,
        &BLUE_AURA,
        &WHITE_EQUIPMENT,
    ])));
    let deck = || vec![table.id_of(&LAND); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

/// `def` on the battlefield, attached to `host` however it got there.
fn attached(g: &mut Game, def: &'static CardDef, host: ObjectId) -> ObjectId {
    let id = put(g, def, Zone::BATTLEFIELD);
    g.state.update_obj(id, |o| o.attached_to = Some(host));
    id
}

fn effect(g: &mut Game, affected: ObjectId, kind: EffectKind) {
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source: affected, controller: YOU, timestamp, affected, kind, duration: Duration::Indefinite, words: Words::NONE });
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The state-based actions performed in a settling frame of the game's own, as the rules perform them.
fn state_based_actions(g: &mut Game) {
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    crate::settle::state_based_actions_alone(&mut cx).expect("nothing is asked");
}

/// Whether `id` is still on the battlefield attached to `host`.
fn stays(g: &Game, id: ObjectId, host: ObjectId) -> bool {
    g.state.obj(id).zone.is(ZoneKind::Battlefield) && g.state.obj(id).attached_to == Some(host)
}

/// Whether `id` was put into its owner's graveyard.
fn went_to_the_graveyard(g: &Game, id: ObjectId) -> bool {
    g.state.obj(id).became.is_some_and(|new| g.state.obj(new).zone.is(ZoneKind::Graveyard))
}

/// White Ward's creature has protection from white: the white Ward stays, another white Aura goes, a blue one is not covered.
/// The protection is the Ward's own source's as much as any other's: it covers the Ward's damage and targeting.
#[test]
fn an_aura_giving_protection_from_its_own_color_stays_and_another_aura_of_that_color_goes() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &WHITE_WARD, bear);
    let white = attached(&mut g, &WHITE_AURA, bear);
    let blue = attached(&mut g, &BLUE_AURA, bear);
    {
        let cx = cx(&mut g);
        let q: &dyn Query = &cx;
        let c = q.chars(bear);
        assert_eq!(c.protection_colors(), ColorSet::of(Color::White));
        assert!(c.protected_from_by(&q.chars(ward), || q.mana_value(ward), YOU), "the Ward's damage and targeting are covered (CR 702.16b, 702.16e)");
        assert!(crate::attach::host_allows(q, ward, bear, false), "where it is, it is legal (CR 702.16n)");
        assert!(!crate::attach::host_allows(q, ward, bear, true), "judged as being attached, it could not be (CR 702.16c)");
    }
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear), "this effect doesn't remove this Aura (CR 702.16n)");
    assert!(went_to_the_graveyard(&g, white), "it removes another white Aura (CR 704.5m)");
    assert!(stays(&g, blue, bear));
}

/// Two White Wards: each one's protection doesn't remove itself, but does remove the other. Both go.
#[test]
fn two_auras_that_each_spare_only_themselves_remove_each_other() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let first = attached(&mut g, &WHITE_WARD, bear);
    let second = attached(&mut g, &WHITE_WARD, bear);
    state_based_actions(&mut g);
    assert!(went_to_the_graveyard(&g, first) && went_to_the_graveyard(&g, second));
}

/// "If the creature has other instances of protection from the same quality, those instances affect Auras as normal"
/// (CR 702.16n): a resolved effect's protection from white removes White Ward.
#[test]
fn another_instance_of_the_same_protection_removes_the_aura() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &WHITE_WARD, bear);
    let black = attached(&mut g, &BLACK_WARD, bear);
    effect(&mut g, bear, EffectKind::Protection(ColorSet::of(Color::White)));
    state_based_actions(&mut g);
    assert!(went_to_the_graveyard(&g, ward), "the other instance removes it");
    assert!(went_to_the_graveyard(&g, black), "Black Ward is white, and so is the other instance");
}

/// Spectra Ward's protection doesn't remove Auras, itself or any other; but "if the enchanted creature gains protection from
/// white in another way, Spectra Ward will be put into its owner's graveyard" (its ruling) - White Ward's, which spares only
/// White Ward, removes Spectra Ward and the other white Aura.
#[test]
fn protection_that_spares_every_aura_removes_none_and_another_instance_still_does() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let spectra = attached(&mut g, &SPECTRA_WARD, bear);
    let white = attached(&mut g, &WHITE_AURA, bear);
    let black = attached(&mut g, &BLACK_AURA, bear);
    state_based_actions(&mut g);
    assert!(stays(&g, spectra, bear) && stays(&g, white, bear) && stays(&g, black, bear), "no Aura is removed");

    let ward = attached(&mut g, &WHITE_WARD, bear);
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear), "Spectra Ward's spares it, and its own does");
    assert!(went_to_the_graveyard(&g, spectra), "White Ward's protection from white removes it");
    assert!(went_to_the_graveyard(&g, white));
    assert!(stays(&g, black, bear), "only Spectra Ward gave protection from black, and it spared every Aura");
}

/// "Although Auras that are already attached to the creature aren't affected by Spectra Ward, the enchanted creature can't be
/// the target of further Aura spells that have one or more colors" (its ruling): nor can a colored Aura be attached to it, or
/// enter attached to it (CR 702.16c, 303.4i), which is no state-based action.
#[test]
fn an_aura_the_protection_would_spare_still_can_not_be_attached() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SPECTRA_WARD, bear);
    let loose = put(&mut g, &WHITE_AURA, Zone::BATTLEFIELD);
    let card = put(&mut g, &WHITE_AURA, Zone::graveyard(YOU));
    assert!(!crate::attach::host_allows(&cx(&mut g), loose, bear, true), "it can't be attached");
    let entered = cantrip_api::Game::put_onto_battlefield_attached(&mut cx(&mut g), card, YOU, bear).expect("nothing is asked");
    assert_eq!(entered, None, "it can't enter attached");
}

/// "Doesn't remove Auras": an Equipment is no Aura, and protection unattaches it as normal (CR 702.16d, 704.5n).
#[test]
fn an_equipment_is_no_aura_and_falls_off() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    attached(&mut g, &SPECTRA_WARD, bear);
    let equipment = attached(&mut g, &WHITE_EQUIPMENT, bear);
    state_based_actions(&mut g);
    assert!(g.state.obj(equipment).zone.is(ZoneKind::Battlefield));
    assert_eq!(g.state.obj(equipment).attached_to, None);
}

/// Black Ward made black in layer 5 (Painter's Servant) has the color its own protection covers: it stays all the same. A white
/// Aura made black goes.
#[test]
fn an_aura_made_the_protected_color_in_layer_five_is_still_spared() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &BLACK_WARD, bear);
    let painted = attached(&mut g, &WHITE_AURA, bear);
    effect(&mut g, ward, EffectKind::SetColors(ColorSet::of(Color::Black)));
    effect(&mut g, painted, EffectKind::SetColors(ColorSet::of(Color::Black)));
    assert_eq!(cx(&mut g).chars(ward).colors, ColorSet::of(Color::Black));
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear), "this effect doesn't remove this Aura, whatever its color");
    assert!(went_to_the_graveyard(&g, painted), "a black Aura is removed");
}

/// Black Ward's "black" changed to "white" (Sleight of Mind, CR 612.2): the creature has protection from white and not black,
/// the white Ward still spared and another white Aura removed; a black Aura is covered no more.
#[test]
fn a_text_change_changes_what_the_protection_covers_and_the_aura_stays() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &BLACK_WARD, bear);
    let white = attached(&mut g, &WHITE_AURA, bear);
    let black = attached(&mut g, &BLACK_AURA, bear);
    effect(&mut g, ward, EffectKind::ChangeText { from: Word::Color(Color::Black), to: Word::Color(Color::White) });
    assert_eq!(cx(&mut g).chars(bear).protection_colors(), ColorSet::of(Color::White));
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear));
    assert!(went_to_the_graveyard(&g, white));
    assert!(stays(&g, black, bear));
}

/// Cho-Manno's Blessing with black chosen: a black Aura goes; the Blessing, white, is not covered; made black, it is spared.
#[test]
fn protection_from_the_chosen_color_spares_its_aura() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &CHOSEN_WARD, bear);
    let text = g.cards.id_of(&CHOSEN_WARD);
    let choices = <cantrip_state::Choices as cantrip_state::ChoiceRecord>::entered(text, cantrip_state::Entered { color: Some(Color::Black), ..Default::default() });
    g.state.update_obj(ward, |o| o.marks.choices = choices);
    let black = attached(&mut g, &BLACK_AURA, bear);
    effect(&mut g, ward, EffectKind::SetColors(ColorSet::of(Color::Black)));
    assert_eq!(cx(&mut g).chars(bear).protection_colors(), ColorSet::of(Color::Black));
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear));
    assert!(went_to_the_graveyard(&g, black));
}

/// Tattoo Ward's protection from enchantments: every other Aura, whatever its color, is an enchantment and goes; Tattoo Ward,
/// one too, stays.
#[test]
fn protection_from_enchantments_spares_its_aura_alone() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &TATTOO_WARD, bear);
    let blue = attached(&mut g, &BLUE_AURA, bear);
    assert_eq!(cx(&mut g).chars(bear).protection_card_types(), TypeSet::of(&[CardType::Enchantment]));
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear));
    assert!(went_to_the_graveyard(&g, blue));
}

/// "Loses all abilities" (CR 613.1f) in layer 6 after White Ward's grant, by timestamp (CR 613.7): the creature has no
/// protection, and a white Aura stays. A Ward attached after the loss gives it again (CR 613.7), sparing itself as before.
#[test]
fn losing_all_abilities_takes_the_protection_and_a_later_grant_gives_it_back() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let ward = attached(&mut g, &WHITE_WARD, bear);
    let white = attached(&mut g, &WHITE_AURA, bear);
    effect(&mut g, bear, EffectKind::LoseAllAbilities);
    assert!(!cx(&mut g).chars(bear).has_protection(), "the granted protection is lost with the rest (CR 613.1f)");
    state_based_actions(&mut g);
    assert!(stays(&g, ward, bear) && stays(&g, white, bear), "no protection removes either");

    let later = attached(&mut g, &WHITE_WARD, bear);
    assert_eq!(cx(&mut g).chars(bear).protection_colors(), ColorSet::of(Color::White), "granted after the loss, it is kept");
    state_based_actions(&mut g);
    assert!(stays(&g, later, bear), "it spares itself");
    assert!(went_to_the_graveyard(&g, ward) && went_to_the_graveyard(&g, white), "and removes the other white Auras");
}

/// The masks are exact for any ids: an instance sparing only `a` spares `a` alone, and two sparing different Auras spare
/// neither, even where one id's bits are a subset of the other's (1 and 3), or one is 0 or every bit.
#[test]
fn what_an_instance_spares_is_exact_for_any_two_ids() {
    let white = ColorSet::of(Color::White);
    let removes = |p: &SparedProtection, aura: u32| p.removing(ObjectId(aura)).0.has(Color::White);
    for (a, b) in [(1, 3), (3, 1), (0, 1), (0, u32::MAX), (u32::MAX, 7), (6, 5)] {
        let mut one = SparedProtection::NONE;
        one.add(white, TypeSet::NONE, Spares::Aura(ObjectId(a)));
        assert!(!removes(&one, a) && removes(&one, b), "{a} alone is spared, {b} is not");
        let mut two = one;
        two.add(white, TypeSet::NONE, Spares::Aura(ObjectId(b)));
        assert!(removes(&two, a) && removes(&two, b), "{a} and {b} remove each other");
        let mut every = one;
        every.add(white, TypeSet::NONE, Spares::Auras);
        assert!(!removes(&every, a) && removes(&every, b), "sparing every Aura does not stop {a}'s removing {b}");
    }
    let mut auras = SparedProtection::NONE;
    auras.add(white, TypeSet::NONE, Spares::Auras);
    assert!(!removes(&auras, 0) && !removes(&auras, u32::MAX));
}
