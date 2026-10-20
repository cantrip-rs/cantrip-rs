//! A player choosing as an effect resolves (CR 608.2d): a card, some cards, or a player, asked as the core's question and
//! answered as the thing chosen. The core checks every answer against its question before it is taken (a card that was
//! not offered, or the wrong number of them, is refused and asked again), so a card's file reads the choice and nothing
//! else; what it does with "nothing to choose" is its own.

use cantrip_api::*;

/// `count` of `eligible` (or up to `count` with `up_to`), chosen from among `shown`, every one of which the player sees:
/// Aesir Escape Valhalla shows the whole graveyard and lets only its permanent cards be chosen. Nothing is asked when
/// nothing is eligible. Fewer than `count` come back only when fewer were eligible, or when the choice is a discard or
/// sacrifice the player can't be made to make (CR 101.2).
pub fn cards_from(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u8,
    up_to: bool,
) -> Result<Vec<ObjectId>, Suspend> {
    cards_from_kind(g, player, source, purpose, "", shown, eligible, count, up_to)
}

/// `cards_from` for a choice the asking card words with more than "card": `kind` is its own filter ("nonland", "artifact or
/// creature"), which the question says to the chooser. Elite Spellbinder and Tidehollow Sculler print "a nonland card".
pub fn cards_from_kind(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    kind: &'static str,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u8,
    up_to: bool,
) -> Result<Vec<ObjectId>, Suspend> {
    if eligible.is_empty() || count == 0 {
        return Ok(Vec::new());
    }
    ask_cards(g, player, RequestKind::ChooseCards { source, purpose, cards: shown, eligible, count, up_to, kind, total: None })
}

/// Up to `count` of `eligible` "with total mana value `most` or less" (CR 202.3), chosen from among `shown`: the mana values
/// of the cards chosen together add up to at most `most`, each read where the card is now (an {X} in a library or graveyard
/// is 0, CR 202.3e). A card worth more than `most` by itself is not eligible; the core refuses a choice whose total is too
/// much. Nothing is asked when nothing is eligible. Michelangelo's Technique (up to two, 6), Lively Dirge (up to two, 4)
/// and Ao, the Dawn Sky (any number, 4) print it.
pub fn cards_within_total(
    g: &mut dyn Game,
    player: PlayerId,
    source: ObjectId,
    purpose: Purpose,
    kind: &'static str,
    shown: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    count: u8,
    most: u32,
) -> Result<Vec<ObjectId>, Suspend> {
    let (eligible, values): (Vec<ObjectId>, Vec<u16>) = eligible
        .into_iter()
        .map(|c| (c, g.mana_value(c)))
        .filter(|&(_, value)| value <= most)
        .map(|(c, value)| (c, value.min(u32::from(u16::MAX)) as u16))
        .unzip();
    if eligible.is_empty() || count == 0 {
        return Ok(Vec::new());
    }
    let total = Some(TotalAtMost { values, most: most.min(u32::from(u16::MAX)) as u16 });
    ask_cards(g, player, RequestKind::ChooseCards { source, purpose, cards: shown, eligible, count, up_to: true, kind, total })
}

/// Asks a `ChooseCards` question, its count no more than there are eligible cards (but "any number", `u8::MAX` up to, stays the
/// number the question was asked with, so it can say "any number of": a client told "up to 2" of two cards could not tell it
/// from "up to two").
fn ask_cards(g: &mut dyn Game, player: PlayerId, mut ask: RequestKind) -> Result<Vec<ObjectId>, Suspend> {
    if let RequestKind::ChooseCards { eligible, count, up_to, .. } = &mut ask {
        if !(*up_to && *count == u8::MAX) {
            *count = (*count).min(eligible.len().min(u8::MAX as usize) as u8);
        }
    }
    match g.ask(player, ask)? {
        Answer::Cards(chosen) => Ok(chosen),
        _ => Ok(Vec::new()),
    }
}

/// `count` of `among` (or up to `count` with `up_to`), all of them shown: `cards_from` with nothing shown but what may be
/// chosen.
pub fn cards(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, among: Vec<ObjectId>, count: u8, up_to: bool) -> Result<Vec<ObjectId>, Suspend> {
    cards_from(g, player, source, purpose, among.clone(), among, count, up_to)
}

/// One of `among`, asked even when the choice is forced: none only when `among` is empty, or when the choice is a discard
/// or sacrifice the player can't be made to make (CR 101.2).
pub fn card(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, among: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    Ok(cards(g, player, source, purpose, among, 1, false)?.first().copied())
}

/// One of `eligible`, chosen from among `shown`: `card` for a choice made in view of cards that may not be chosen.
pub fn card_from(g: &mut dyn Game, player: PlayerId, source: ObjectId, purpose: Purpose, shown: Vec<ObjectId>, eligible: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    Ok(cards_from(g, player, source, purpose, shown, eligible, 1, false)?.first().copied())
}

/// "An opponent chooses one of them": the spell or ability's controller says which opponent chooses - the effect names no
/// opponent of its own, so which one it is is a choice it leaves to them as it applies (CR 608.2d), asked only when they
/// have more than one - and that opponent names one of `cards`, every one of which they see. The one card is taken without
/// asking, and `None` comes back for no cards at all or for a controller with no opponent. Karn, Scion of Urza and
/// Extrapolate the Impossible print it.
pub fn opponent_chooses_one(g: &mut dyn Game, controller: PlayerId, source: ObjectId, cards: Vec<ObjectId>) -> Result<Option<ObjectId>, Suspend> {
    if cards.len() <= 1 {
        return Ok(cards.first().copied());
    }
    let opponents: Vec<PlayerId> = g.state().opponents(controller).collect();
    let chooser = match opponents.len() {
        0 => return Ok(None),
        1 => opponents[0],
        _ => match player(g, controller, source, opponents, "ChooseRevealedCard", 0)? {
            Some(chooser) => chooser,
            None => return Ok(None),
        },
    };
    card(g, chooser, source, Purpose::PutIntoHand, cards)
}

/// A player of `candidates`, chosen without targeting (CR 608.2d, 614.12a), asked even when the choice is forced; none
/// only when there are no candidates. `verb` and `amount` say what the chosen player will do, as hint text.
pub fn player(g: &mut dyn Game, chooser: PlayerId, source: ObjectId, candidates: Vec<PlayerId>, verb: &'static str, amount: u16) -> Result<Option<PlayerId>, Suspend> {
    if candidates.is_empty() {
        return Ok(None);
    }
    match g.ask(chooser, RequestKind::ChoosePlayer { source, candidates, verb, amount })? {
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

/// The face a card is cast as (CR 709.3, 715.3, 712.8a), chosen from `ways` (`Game::cast_ways_during_resolution`, narrowed to
/// the ones the effect allows: a mana value, a type), as the player puts it on the stack. Nothing is asked when there is one;
/// `None` when there is none. Uldaros Theorix, Beseech the Mirror and Bilbo, Thief in the Night print a restriction on the spell.
pub fn cast_way(g: &mut dyn Game, player: PlayerId, card: ObjectId, ways: Vec<CastWay>) -> Result<Option<CastWay>, Suspend> {
    match ways.len() {
        0 => Ok(None),
        1 => Ok(Some(ways[0])),
        _ => {
            let faces = ways.iter().map(|w| Label::Face(w.face)).collect();
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
    let ways_of = |g: &dyn Game, card: ObjectId| -> Vec<CastWay> {
        g.cast_ways_during_resolution(card).into_iter().filter(|w| fits(g, w)).collect()
    };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let eligible: Vec<ObjectId> = hand.iter().copied().filter(|&c| !ways_of(&*g, c).is_empty()).collect();
    let Some(&card) = cards_from_kind(g, player, source, Purpose::CastFree, kind, hand, eligible, 1, true)?.first() else {
        return Ok(());
    };
    let ways = ways_of(&*g, card);
    let way = cast_way(g, player, card, ways)?.map(|w| w.way);
    g.cast_during_resolution_as(card, CastTerms { instead: Some(ManaCost::ZERO), way, ..CastTerms::default() })?;
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
        let castable: Vec<ObjectId> = cards.iter().copied().filter(|&c| g.could_cast_during_resolution(c)).collect();
        let chosen = cards_from(g, player, source, Purpose::CastFree, cards.to_vec(), castable, 1, true)?;
        let Some(&card) = chosen.first() else { return Ok(()) };
        g.cast_during_resolution(card, Some(ManaCost::ZERO))?;
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
/// Siege), the Tarkir: Dragonstorm Sieges ("Jeskai or Temur": Frostcliff Siege; Barrensteppe, Glacierwood, Hollowmurk and
/// Windcrag Siege), Mirrodin Besieged ("Mirran or Phyrexian"), Gollum, Riddle Master and Lavabrink Venturer ("odd or even")
/// print it. The linked abilities read it with `chosen_option`.
pub fn option_as_enters(g: &mut dyn Game, entering: &mut Entering, def: &'static CardDef, options: core::ops::Range<u8>) -> Result<(), Suspend> {
    let card = g.card_id(def);
    let index = option(g, entering.controller, entering.object, card, options)?;
    entering.chosen_option = Some(ChosenOption { card, index });
    Ok(())
}

/// The word chosen as `id` entered (`option_as_enters`), by index into `def`'s `labels`, for "the chosen [word]" or "• Jeskai
/// — ..." of `def`'s text: none if no choice was made, or if it was made by another card's ability, which `def`'s abilities
/// are not linked to (CR 607.2f): a permanent that entered as one card and has since become a copy of another.
pub fn chosen_option(q: &dyn Query, id: ObjectId, def: &'static CardDef) -> Option<u8> {
    q.obj(id).marks.chosen_option.filter(|c| std::ptr::eq(q.card(c.card), def)).map(|c| c.index)
}

/// The labels "choose odd or even" offers, odd first: a card that asks it lists them as its `labels`.
pub const ODD_OR_EVEN: &[&str] = &["Odd", "Even"];

/// "Choose odd or even" (Thanos, the Mad Titan's power-up, as it resolves): true for odd. `card` is the card whose `labels`
/// are `ODD_OR_EVEN`. "As this enters, choose odd or even" is `option_as_enters`.
pub fn odd_or_even(g: &mut dyn Game, player: PlayerId, source: ObjectId, card: CardId) -> Result<bool, Suspend> {
    Ok(option(g, player, source, card, 0..2)? == 0)
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
    q.state().obj(id).marks.chosen_option.filter(|c| std::ptr::eq(q.cards().get(c.card), def)).map(|c| c.index)
}
