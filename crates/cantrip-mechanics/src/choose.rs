//! A player choosing as an effect resolves (CR 608.2d): a card, some cards, or a player, asked as the core's question and
//! answered as the thing chosen. The core checks every answer against its question before it is taken (a card that was
//! not offered, or a set its rule refuses, is refused and asked again), so a card's file reads the choice and nothing
//! else; what it does with "nothing to choose" is its own.
//!
//! **A choice of cards is `select`**, with a rule from `crate::select` ("up to two creature cards", "two cards with
//! different names"). The older functions below that take a list of eligible cards and a count (`card`, `cards`,
//! `card_from`, `cards_from`, `cards_from_kind`, `any_number_of`, `cards_within_total`) are kept for existing cards; new
//! cards use `select`.

use crate::select::{total_mana_value_at_most, Count, Selection};
use cantrip_api::*;

/// "Choose [cards]": the cards `rule` allows (`crate::select`: how many, of what quality, together how), chosen from among
/// `cards`, every one of which the player sees - a revealed hand, the cards looked at, a graveyard. Asked whenever there are
/// cards to show, even when none of them may be chosen (seeing them is part of the effect: Duress); a caller with nothing
/// to show asks nothing. The cards chosen come back in the order chosen; none when the choice is a discard or sacrifice the
/// player can't be made to make (CR 101.2).
pub fn select(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, rule: Selection) -> Result<Vec<ObjectId>, Suspend> {
    if cards.is_empty() {
        return Ok(Vec::new());
    }
    match g.ask(player, select_question(source, purpose, cards, rule))? {
        Answer::Cards(chosen) => Ok(chosen),
        _ => Ok(Vec::new()),
    }
}

/// The question `select` asks, for a card written as explicit stages, which asks it with `Game::ask_then` (Duress).
pub fn select_question(source: ObjectId, purpose: Purpose, cards: Vec<ObjectId>, rule: Selection) -> RequestKind {
    RequestKind::ChooseCards { source, purpose, cards, rule: Box::new(rule) }
}

/// The count an older choice of `count` (or up to `count`) of `eligible` asks: no more than there are eligible cards.
fn listed_count(eligible: &[ObjectId], count: u32, up_to: bool) -> Count {
    let count = count.min(count_u32(eligible.len()));
    if up_to { Count::UpTo(count) } else { Count::Exactly(count) }
}

/// `count` of `eligible` (or up to `count` with `up_to`), chosen from among `shown`, every one of which the player sees:
/// Aesir Escape Valhalla shows the whole graveyard and lets only its permanent cards be chosen. Nothing is asked when
/// nothing is eligible. Fewer than `count` come back only when fewer were eligible, or when the choice is a discard or
/// sacrifice the player can't be made to make (CR 101.2). Kept for existing cards; new cards use `select`.
pub fn cards_from(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u32,
    up_to: bool,
) -> Result<Vec<ObjectId>, Suspend> {
    cards_from_kind(g, player, source, purpose, "", shown, eligible, count, up_to)
}

/// `cards_from` for a choice the asking card words with more than "card": `kind` is its own filter ("nonland", "artifact or
/// creature"), which the question says to the chooser. Elite Spellbinder and Tidehollow Sculler print "a nonland card".
/// Kept for existing cards; new cards use `select`.
pub fn cards_from_kind(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    kind: &'static str,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u32,
    up_to: bool,
) -> Result<Vec<ObjectId>, Suspend> {
    if eligible.is_empty() || count == 0 {
        return Ok(Vec::new());
    }
    let rule = Selection::listed(listed_count(&eligible, count, up_to), &shown, eligible).called(kind);
    select(g, player, source, purpose, shown, rule)
}

/// "Any number of" `eligible` (CR 608.2d: none, some or all), chosen from among `shown`, every one of which the player sees,
/// the choice worded as the card words it (`kind`, empty for "card"). Nothing is asked when nothing is eligible. Kept for
/// existing cards; new cards use `select` with `select::ANY_NUMBER`.
pub fn any_number_of(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    kind: &'static str,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
) -> Result<Vec<ObjectId>, Suspend> {
    if eligible.is_empty() {
        return Ok(Vec::new());
    }
    let rule = Selection::listed(crate::select::ANY_NUMBER, &shown, eligible).called(kind);
    select(g, player, source, purpose, shown, rule)
}

/// Up to `count` of `eligible` "with total mana value `most` or less" (CR 202.3, `select::total_mana_value_at_most`), chosen
/// from among `shown`. A card worth more than `most` by itself is not eligible; the core refuses a choice whose total is too
/// much. Nothing is asked when nothing is eligible. Michelangelo's Technique (up to two, 6), Lively Dirge (up to two, 4)
/// and Ao, the Dawn Sky (any number, 4) print it. Kept for existing cards; new cards use `select` with that part.
pub fn cards_within_total(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    kind: &'static str,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u32,
    most: u32,
) -> Result<Vec<ObjectId>, Suspend> {
    let eligible: Vec<ObjectId> = eligible.into_iter().filter(|&c| g.mana_value(c) <= most).collect();
    if eligible.is_empty() || count == 0 {
        return Ok(Vec::new());
    }
    let rule = Selection::listed(listed_count(&eligible, count, true), &shown, eligible).called(kind).with(total_mana_value_at_most(most));
    select(g, player, source, purpose, shown, rule)
}

/// `count` of `among` (or up to `count` with `up_to`), all of them shown: `cards_from` with nothing shown but what may be
/// chosen. Kept for existing cards; new cards use `select`.
pub fn cards(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, among: Vec<ObjectId>, count: u32, up_to: bool) -> Result<Vec<ObjectId>, Suspend> {
    cards_from(g, player, source, purpose, among.clone(), among, count, up_to)
}

/// One of `among`, asked even when the choice is forced: none only when `among` is empty, or when the choice is a discard
/// or sacrifice the player can't be made to make (CR 101.2). Kept for existing cards; new cards use `select`.
pub fn card(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, among: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    Ok(cards(g, player, source, purpose, among, 1, false)?.first().copied())
}

/// One of `eligible`, chosen from among `shown`: `card` for a choice made in view of cards that may not be chosen. Kept for
/// existing cards; new cards use `select`.
pub fn card_from(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, shown: Vec<ObjectId>, eligible: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    Ok(cards_from(g, player, source, purpose, shown, eligible, 1, false)?.first().copied())
}

/// "An opponent chooses `count` of them": the spell or ability's controller says which opponent chooses - the effect names no
/// opponent of its own, so which one it is is a choice it leaves to them as it applies (CR 608.2d), asked only when they
/// have more than one - and that opponent names `count` of `cards` (all of them when there are no more), every one of which
/// they see. Nothing is asked when the choice is forced, and nothing comes back for no cards at all or for a controller with
/// no opponent. Turtles Forever (two) prints it; `opponent_chooses_one` is Karn, Scion of Urza's and Extrapolate the
/// Impossible's.
pub fn opponent_chooses(g: &mut dyn Game, controller: PlayerId, source: ObjectId, cards: Vec<ObjectId>, count: u32) -> Result<Vec<ObjectId>, Suspend> {
    if cards.len() <= count as usize {
        return Ok(cards);
    }
    let opponents: Vec<PlayerId> = g.state().opponents(controller).collect();
    let chooser = match opponents.len() {
        0 => return Ok(Vec::new()),
        1 => opponents[0],
        _ => match player(g, controller, source, opponents, "ChooseRevealedCard", 0)? {
            Some(chooser) => chooser,
            None => return Ok(Vec::new()),
        },
    };
    self::cards(g, chooser, source, Purpose::PutIntoHand, cards, count, false)
}

/// "An opponent chooses one of them" (`opponent_chooses`): the one card, taken without asking when it is the only one.
/// Karn, Scion of Urza and Extrapolate the Impossible print it.
pub fn opponent_chooses_one(g: &mut dyn Game, controller: PlayerId, source: ObjectId, cards: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    Ok(opponent_chooses(g, controller, source, cards, 1)?.first().copied())
}

/// A player of `candidates`, chosen without targeting (CR 608.2d, 614.12a), asked even when the choice is forced; none
/// only when there are no candidates. `verb` and `amount` say what the chosen player will do, as hint text.
pub fn player(g: &mut dyn Game, chooser: PlayerId, source: ObjectId, candidates: Vec<PlayerId>, verb: &'static str, amount: u16) -> Result<Option<PlayerId>, Suspend> {
    if candidates.is_empty() {
        return Ok(None);
    }
    match g.ask(chooser, RequestKind::ChoosePlayer { source, candidates, verb, amount, subject: None })? {
        Answer::Player(p) => Ok(Some(p)),
        _ => Ok(None),
    }
}

/// One of the labelled options `modes`, by index (a pile, a mode, "tap or untap"), asked even when the choice is forced.
pub fn one_of(g: &mut dyn Game, player: PlayerId, source: ObjectId, modes: Vec<Label>) -> Result<u8, Suspend> {
    let eligible = (0..modes.len() as u8).collect();
    let ask = RequestKind::ChooseModes { source, modes, eligible, count: 1, at_most: 0, affordable: Vec::new(), additional_cost: false, repeat: false };
    match g.ask(player, ask)? {
        Answer::Modes(chosen) => Ok(chosen.first().copied().unwrap_or(0)),
        _ => Ok(0),
    }
}

/// "Separate [these] into two piles" (CR 700.3a): `separator` chooses the cards of the first pile and the rest are the second,
/// so either pile may be empty (CR 700.3d); the two piles come back in that order. Nothing is asked when there is
/// nothing to separate, or when no player is there to separate (the caller's whole pile stays second). Fact or Fiction and
/// Liliana of the Veil print it.
pub fn separate(g: &mut dyn Game, separator: Option<PlayerId>, source: ObjectId, cards: Vec<ObjectId>) -> Result<(Vec<ObjectId>, Vec<ObjectId>), Suspend> {
    let Some(separator) = separator else { return Ok((Vec::new(), cards)) };
    let first = select(g, separator, source, Purpose::SeparatePiles, cards.clone(), crate::select::cards(crate::select::ANY_NUMBER))?;
    let second = cards.into_iter().filter(|id| !first.contains(id)).collect();
    Ok((first, second))
}

/// "Choose a pile" during resolution (CR 608.2d, 700.3b-d): name both piles and whether each is empty. Return the chosen
/// pile first and the other second. Fact or Fiction and Liliana of the Veil print it.
pub fn pile(g: &mut dyn Game, chooser: PlayerId, source: ObjectId, first: Vec<ObjectId>, second: Vec<ObjectId>) -> Result<(Vec<ObjectId>, Vec<ObjectId>), Suspend> {
    let piles = vec![Label::Pile { first: true, cards: first.clone() }, Label::Pile { first: false, cards: second.clone() }];
    Ok(if one_of(g, chooser, source, piles)? == 1 { (second, first) } else { (first, second) })
}

/// "Look at [these] and separate them into a face-down pile and a face-up pile" (CR 700.3a): `separator` looks at the cards
/// and chooses those of the face-down pile, the rest being the face-up pile, so either pile may be empty (CR 700.3d); the two
/// piles come back in that order. Nothing is asked when there is nothing to separate. The face-up cards are then revealed to
/// every player (CR 701.20a, the glossary's "Face Up"), the face-down ones not: the card moves the face-up pile with the
/// `_revealed` verbs. Riddles in the Dark, Curator of Destinies
/// (a face-down and a face-up pile) and Fortune's Favor and Atris, Oracle of Half-Truths (an opponent separates) print it.
pub fn separate_face_down_and_face_up(g: &mut dyn Game, separator: PlayerId, source: ObjectId, cards: Vec<ObjectId>) -> Result<(Vec<ObjectId>, Vec<ObjectId>), Suspend> {
    let face_down = select(g, separator, source, Purpose::SeparateFaceDown, cards.clone(), crate::select::cards(crate::select::ANY_NUMBER))?;
    let face_up: Vec<ObjectId> = cards.into_iter().filter(|id| !face_down.contains(id)).collect();
    g.reveal(separator, &face_up);
    Ok((face_down, face_up))
}

/// "Choose one of the piles" of a face-down pile and a face-up pile during resolution (CR 608.2d, 700.3b-d): the chooser sees
/// what the face-up pile holds and how many cards the face-down pile does, not what they are. Returns the chosen pile first and
/// the other second. Riddles in the Dark, Curator of Destinies, Fortune's Favor and Atris, Oracle of Half-Truths print it.
pub fn face_down_or_face_up_pile(
    g: &mut dyn Game,
    chooser: PlayerId,
    source: ObjectId,
    face_down: Vec<ObjectId>,
    face_up: Vec<ObjectId>,
) -> Result<(Vec<ObjectId>, Vec<ObjectId>), Suspend> {
    let piles = vec![Label::FaceUpPile { cards: face_up.clone() }, Label::FaceDownPile { count: face_down.len() as u16 }];
    Ok(if one_of(g, chooser, source, piles)? == 1 { (face_down, face_up) } else { (face_up, face_down) })
}

/// The face a card is cast as (CR 709.3, 715.3, 712.8a), chosen from `ways` (`Game::cast_ways_during_resolution`, narrowed to
/// the ones the effect allows: a mana value, a type), as the player puts it on the stack. Nothing is asked when there is one;
/// `None` when there is none. Uldaros Theorix, Beseech the Mirror and Bilbo, Thief in the Night print a restriction on the spell.
pub fn cast_way(g: &mut dyn Game, player: PlayerId, card: ObjectId, ways: Vec<CastWay>) -> Result<Option<CastWay>, Suspend> {
    match ways.len() {
        0 => Ok(None),
        1 => Ok(Some(ways[0])),
        _ => {
            let faces = ways.iter().map(|w| if w.way == WAY_FUSED { Label::Fused(w.face) } else { Label::Face(w.face) }).collect();
            Ok(Some(ways[one_of(g, player, card, faces)? as usize]))
        }
    }
}

/// "You may cast a spell from your hand [that `fits`] without paying its mana cost" as an effect resolves (CR 608.2g, 118.9):
/// the controller chooses a card in their hand with a way to cast it that `fits` the words (a mana value, a type of the face
/// that would be cast, CR 709.3a, 715.3a), then the way if the card has several, and casts it for nothing. Nothing is asked when
/// no card in hand qualifies; `kind` is how the question words the card ("artifact", or "" for any). Buster Sword and Lady
/// Octopus, Inspired Inventor print it.
pub fn may_cast_from_hand_free(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    kind: &'static str,
    fits: impl Fn(&dyn Game, &CastWay) -> bool,
) -> Result<(), Suspend> {
    let ways_of = |g: &mut dyn Game, card: ObjectId| -> Vec<CastWay> {
        let ways = g.cast_ways_during_resolution_by(card, player);
        ways.into_iter().filter(|w| fits(&*g, w)).collect()
    };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let mut eligible: Vec<ObjectId> = Vec::new();
    for &c in &hand {
        if !ways_of(&mut *g, c).is_empty() {
            eligible.push(c);
        }
    }
    let Some(&card) = cards_from_kind(g, player, source, Purpose::CastFree, kind, hand, eligible, 1, true)?.first() else {
        return Ok(());
    };
    let ways = ways_of(&mut *g, card);
    let way = cast_way(g, player, card, ways)?.map(|w| w.way);
    g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), way, caster: Some(player), ..CastTerms::default() })?;
    Ok(())
}

/// "You may cast any number of spells from among [these cards] without paying their mana costs" (CR 608.2g, 118.9): the
/// controller is offered the castable ones among `cards` one at a time, in the order they like, until they decline or none
/// is castable. Each is cast as the effect resolves, so it takes its face (when it has several) and its targets as any cast
/// does; a cast given up leaves the card where it was, to be offered again. A card is castable only while it is still
/// where it was (`could_cast_during_resolution` asks), so one already cast is not offered.
/// Etali, Primal Conqueror and Improvisation Capstone print it.
pub fn cast_any_number_free(g: &mut dyn Game, player: PlayerId, source: ObjectId, cards: &[ObjectId]) -> Result<(), Suspend> {
    cast_up_to_free(g, player, source, cards, usize::MAX)
}

/// "You may cast up to `most` spells from among [these cards] without paying their mana costs": as `cast_any_number_free`,
/// nothing more offered once `most` have been cast; a cast given up does not count. Doom Reigns Supreme uses it with
/// two; Plargg and Nassari (not in content yet) is the second printed card with the same words.
pub fn cast_up_to_free(g: &mut dyn Game, player: PlayerId, source: ObjectId, cards: &[ObjectId], most: usize) -> Result<(), Suspend> {
    let mut cast = 0;
    while cast < most {
        let castable: Vec<ObjectId> = cards.iter().copied().filter(|&c| g.could_cast_during_resolution_by(c, player)).collect();
        let chosen = cards_from(g, player, source, Purpose::CastFree, cards.to_vec(), castable, 1, true)?;
        let Some(&card) = chosen.first() else { return Ok(()) };
        g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), caster: Some(player), ..CastTerms::default() })?;
        if g.obj(card).became.is_some() {
            cast += 1;
        }
    }
    Ok(())
}

/// "Choose A or B": one of the words a card's text offers to choose between, words that otherwise have no rules meaning
/// (CR 607.2f), by index into its `labels`: `options` are the indices of the words offered, in the card's order. Asked even
/// when the choice is forced; the question is captioned with every word offered ("Frostcliff Siege: choose Jeskai or Temur").
/// Thanos, the Mad Titan's "choose odd or even" asks it as his power-up resolves.
pub fn option(g: &mut dyn Game, player: PlayerId, source: ObjectId, card: CardId, options: core::ops::Range<u8>) -> Result<u8, Suspend> {
    let start = options.start;
    Ok(start + one_of(g, player, source, options.map(|i| Label::Option(card, i)).collect())?)
}

/// "As [this] enters, choose A or B" (CR 614.12a, 607.2f): the controller chooses one of the words `def`'s text offers (the
/// `options` of its `labels`) before it enters, and the permanent keeps the choice while it is on the battlefield; it is
/// no copiable value (CR 707.2), so a copy entering makes its own choice (CR 707.5). `def` is the card whose text this is,
/// the caller's `&CARD`. The Khans of Tarkir Sieges ("Khans or Dragons": Monastery, Outpost, Palace, Citadel and Frontier
/// Siege), the Tarkir: Dragonstorm Sieges ("Jeskai or Temur": Frostcliff Siege; "Mardu or Jeskai": Windcrag Siege; the other
/// Tarkir: Dragonstorm Sieges name their own pair), Mirrodin Besieged ("Mirran or Phyrexian"), Gollum, Riddle Master and Lavabrink Venturer ("odd or even")
/// print it. The linked abilities read it with `chosen_option`.
pub fn option_as_enters(g: &mut dyn Game, entering: &mut Entering, def: &'static CardDef, options: core::ops::Range<u8>) -> Result<(), Suspend> {
    let card = g.card_id(def);
    let index = option(g, entering.controller, entering.object, card, options)?;
    entering.chosen_option = Some(ChosenOption { card, index });
    Ok(())
}

/// "Choose a number between `least` and `most`" (CR 107.1, 614.12a): endpoints are included, and the choice is asked even
/// when only one value is legal. `heading` is the card's named ability or instruction. Expel the Interlopers and Nyota
/// Uhura, Communications print this choice.
pub fn number(g: &mut dyn Game, player: PlayerId, source: ObjectId, least: u32, most: u32, heading: &'static str) -> Result<u32, Suspend> {
    assert!(least <= most && most - least <= u8::MAX.into(), "a number choice must contain between 1 and 256 values");
    match g.ask(player, RequestKind::ChooseNumber { source, least, most, heading })? {
        Answer::Announce(value) => Ok(value),
        _ => unreachable!("the core validates answers to ChooseNumber"),
    }
}

/// "As [this] enters, choose a number between `least` and `most" (CR 614.12a, 607.2d): the permanent keeps the chosen
/// value for its own linked abilities, including when the same card name is a different object after changing zones.
pub fn number_as_enters(g: &mut dyn Game, entering: &mut Entering, least: u32, most: u32, heading: &'static str) -> Result<(), Suspend> {
    entering.chosen_number = Some(number(g, entering.controller, entering.object, least, most, heading)?);
    Ok(())
}

/// The number chosen as this permanent entered, read by an ability of the text that chose it (CR 607.2d, 607.5).
pub fn chosen_number(q: &dyn Query, asker: &Asker) -> Option<u32> {
    q.state().chosen_number(q.linked_to(asker))
}

/// The word chosen as `asker`'s source entered (`option_as_enters`), by index into `def`'s `labels`, for "the chosen [word]" or
/// "• Jeskai — ..." of `def`'s text, read by the ability that asks (`Query::linked_to`): none if no choice was made, or if it
/// was made by an ability that one is not linked to (CR 607.2f, 607.5) - another card's, its own before a copy effect gave
/// it this text, or the permanent's own where an effect gave it the ability asking.
pub fn chosen_option(q: &dyn Query, asker: &Asker, def: &'static CardDef) -> Option<u8> {
    q.state().chosen_option(q.linked_to(asker)).filter(|c| std::ptr::eq(q.card(c.card), def)).map(|c| c.index)
}

/// "As [this] enters, choose [a permanent]" (CR 614.12a, 607.2d): before it enters, its controller chooses one of the
/// permanents `rule` allows among `permanents` (the noun and count are the rule's: `select::up_to(1)` for "you may
/// choose"), and the permanent keeps the choice for as long as it is on the battlefield. Only what is on the battlefield as
/// the choice is made may be chosen: a permanent entering at the same time is not there yet (CR 614.12a; Dauntless
/// Bodyguard's and Metamorphic Alteration's rulings), nor is the one entering. Nobody is asked when `rule` allows none of
/// them, and nothing is chosen. It is not targeting (CR 115.10), so hexproof and shroud don't stop it. Only the abilities
/// of the text that asks refer to the choice (CR 607.2d, `chosen_object`): the engine keeps it with that text. The
/// choice is no copiable value (CR 707.2), so a copy entering makes its own (CR 707.5). Dauntless Bodyguard and Tyrannical
/// Pitlord ("another creature you control"), Scheming Fence ("you may choose a nonland permanent") and Metamorphic
/// Alteration ("a creature") print it.
pub fn object_as_enters(g: &mut dyn Game, entering: &mut Entering, permanents: Vec<ObjectId>, rule: Selection) -> Result<(), Suspend> {
    let (chooser, source) = (entering.controller, entering.object);
    let permanents: Vec<ObjectId> = permanents.into_iter().filter(|&id| id != source && g.obj(id).zone.is(ZoneKind::Battlefield)).collect();
    let none: &[ObjectId] = &[];
    if !cantrip_api::select::judge(&*g, &rule, &cantrip_api::select::Choice { chooser, source, candidates: &permanents, selected: none }).addable.any() {
        return Ok(());
    }
    let chosen = select(g, chooser, source, Purpose::ChooseAsEnters, permanents, rule)?;
    entering.chosen_object = chosen.first().copied();
    Ok(())
}

/// "The chosen [permanent]" of `by`'s text (CR 607.2d): the permanent chosen as `by`'s source entered (`object_as_enters`),
/// read by an ability linked to the one that chose it - `Query::linked` of a resolving ability, `Query::linked_to` of a
/// static one - from its source as it is or as it last existed, if it has left (CR 608.2h). None if nothing was chosen, if
/// `by` is not linked to that choice (CR 607.5: an ability of another text, or one a copy or grant effect gave it later), or
/// if the chosen permanent is no longer on the battlefield: one that left is gone, and if it has come back it is a new
/// object (CR 400.7) that was never chosen; one phased out is treated as though it doesn't exist until it phases in
/// (CR 702.26b).
pub fn chosen_object(q: &dyn Query, by: Linked) -> Option<ObjectId> {
    let chosen = q.state().chosen_object(by)?;
    let obj = q.obj(chosen);
    (obj.became.is_none() && obj.zone.is(ZoneKind::Battlefield)).then_some(chosen)
}

/// The labels "choose odd or even" offers, odd first: a card that asks it lists them as its `labels`.
pub const ODD_OR_EVEN: &[&str] = &["Odd", "Even"];

/// "Choose odd or even" (Thanos, the Mad Titan's power-up, as it resolves): true for odd. `card` is the card whose `labels`
/// are `ODD_OR_EVEN`. "As this enters, choose odd or even" is `option_as_enters`.
pub fn odd_or_even(g: &mut dyn Game, player: PlayerId, source: ObjectId, card: CardId) -> Result<bool, Suspend> {
    Ok(option(g, player, source, card, 0..2)? == 0)
}

/// "Choose odd or even. [Do something to] each creature with mana value of the chosen quality" (Extinction Event, Thanos, the
/// Mad Titan): the choice is made as the spell or ability resolves (CR 608.2d), and the creatures of that quality, any
/// player's and the source's own included, are returned for the caller to act on all at once. `card` is the card whose
/// `labels` are `ODD_OR_EVEN`.
pub fn creatures_of_chosen_quality(g: &mut dyn Game, item: &StackItem, card: CardId) -> Result<Vec<ObjectId>, Suspend> {
    let odd = odd_or_even(g, item.controller, item.object, card)?;
    Ok(g.state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && mana_value_of_quality(g, id, odd))
        .collect())
}

/// The labels "you may tap or untap [it]" offers: a card that asks it lists them as its `labels`.
pub const TAP_OR_UNTAP: &[&str] = &["Tap", "Untap", "Do nothing"];

/// "You may tap or untap target [permanent]" (CR 701.26): the target chosen as the ability went on the stack, and as it
/// resolves its controller chooses Tap, Untap or Do nothing (CR 608.2d); nothing if the target is no longer legal (CR
/// 608.2b). `card` is the card (the face) whose `labels` are `TAP_OR_UNTAP`. Sewer-veillance Cam and Iceberg Titan print it.
pub fn may_tap_or_untap_target(g: &mut dyn Game, item: &StackItem, card: CardId) -> Result<(), Suspend> {
    let Some(permanent) = crate::targets::object(g, item, 0) else { return Ok(()) };
    match one_of(g, item.controller, item.object, vec![Label::Card(card, 0), Label::Card(card, 1), Label::Card(card, 2)])? {
        0 => g.tap(permanent),
        1 => g.untap(permanent),
        _ => {}
    }
    Ok(())
}

/// Whether an object's mana value is of the chosen quality, odd or even: zero is even (Gollum, Riddle Master; Thanos, the
/// Mad Titan).
pub fn mana_value_of_quality(q: &dyn Query, id: ObjectId, odd: bool) -> bool {
    (q.mana_value(id) % 2 == 1) == odd
}

/// `chosen_option` for a static ability's `affects`, which reads through the layers (CR 613): a Siege's "• Temur — Creatures
/// you control get +1/+0 ..." applies only while its source has the word chosen.
pub fn chosen_option_in_layers(q: &dyn LayerQuery, id: ObjectId, def: &'static CardDef) -> Option<u8> {
    q.state().chosen_option(q.linked_to(id)).filter(|c| std::ptr::eq(q.cards().get(c.card), def)).map(|c| c.index)
}
