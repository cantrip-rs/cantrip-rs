//! What lands and mana sources print beyond their mana abilities, which a card writes as
//! `mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")` (`cantrip_api::ability`).

use cantrip_api::*;

/// "Add {U} for each artifact you control" (CR 106.5): counted as the ability resolves, possibly none. A mana ability's
/// `add_by`: Tolarian Academy, and Uthros, Titanic Godcore's 12+ ability.
pub fn add_blue_for_each_artifact(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let artifacts = crate::artifacts_you_control(g, asker.controller);
    if artifacts > 0 {
        g.add_mana(asker.controller, Mana::U, artifacts as u32);
    }
    Ok(())
}

/// "Add {G} for each creature you control" (CR 106.5): counted as the ability resolves, possibly none. A mana ability's
/// `add_by`: Gaea's Cradle, and Evendo, Waking Haven's 12+ ability.
pub fn add_green_for_each_creature(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let creatures = g.permanents_of(asker.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).count();
    if creatures > 0 {
        g.add_mana(asker.controller, Mana::G, creatures as u32);
    }
    Ok(())
}

/// "Add one mana of any of the exiled cards' colors" (CR 106.5, 607.2a): the colors the cards exiled with the source have now, for a
/// mana ability's `colors_from`. With no exiled card, or only colorless ones, there is no color to choose: the ability is still
/// offered and adds no mana (CR 106.5). Chrome Mox ("the exiled card") and Pit of Offerings read it.
pub fn colors_of_cards_exiled_with_this(q: &dyn Query, asker: &Asker) -> ColorSet {
    let mut colors = ColorSet::NONE;
    for card in q.exiled_with(q.linked_to(asker)) {
        colors = ColorSet(colors.0 | q.chars(card).colors.0);
    }
    colors
}

/// "This land enters tapped" (CR 614.1d): the modal double-faced lands' (Bala Ged Sanctuary, Haven of the Harvest).
pub fn enters_tapped(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    Ok(())
}

/// "This creature enters tapped unless it's your turn" / "enters tapped if it's not your turn" (CR 614.1d): it only ever sets
/// tapped, so a permanent an instruction puts onto the battlefield tapped stays tapped. It is its controller's turn when the
/// active player is the one it enters under. Eddymurk Crab and Horned Loch-Whale print it.
pub fn enters_tapped_unless_your_turn(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.state().turn.active != entering.controller {
        entering.tapped = true;
    }
    Ok(())
}

/// "As this land enters, you may pay 2 life. If you don't, it enters tapped" (CR 614.12a): the shocklands'. Only a
/// player with the life to pay is asked (CR 119.4, `Game::may_pay`). Blood Crypt and Breeding Pool print it.
pub fn enters_tapped_unless_pay_two_life(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if !g.may_pay(entering.controller, entering.object, entering.object, life(2), &PAY_TWO_LIFE_OR_TAPPED)? {
        entering.tapped = true;
    }
    Ok(())
}

/// "As this land enters, you may pay 3 life. If you don't, it enters tapped" (CR 614.12a): the modal double-faced
/// pain lands'. Soporific Springs and Shatterskull, the Hammer Pass print it.
pub fn enters_tapped_unless_pay_three_life(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if !g.may_pay(entering.controller, entering.object, entering.object, life(3), &PAY_THREE_LIFE_OR_TAPPED)? {
        entering.tapped = true;
    }
    Ok(())
}

/// Adds one mana of the color chosen as the ability was activated (CR 106.1a), then the source deals 1 damage to its
/// controller, all at once (a mana ability: no stack). The Talismans print it, every one of the five enemy-color pairs:
/// `mana_ability(TAP, add_one_of(&[a, b]).by(mana::add_chosen_and_hurt), text)`.
pub fn add_chosen_and_hurt(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    if let Some(c) = color {
        g.add_mana(asker.controller, c.mana(), 1);
    }
    g.damage(asker.source, Target::Player(asker.controller), 1);
    Ok(())
}

/// How many other lands the entering land's controller controls, read on the battlefield as it is while the land enters,
/// so the land itself is not counted (CR 614.1d).
pub fn other_lands_you_control(g: &mut dyn Game, entering: &Entering) -> usize {
    g.permanents_of(entering.controller)
        .into_iter()
        .filter(|&id| g.obj(id).became.is_none() && g.chars(id).is(CardType::Land))
        .count()
}

/// "If you control two or more other lands, this land enters tapped" (CR 614.1d): read on the battlefield as it is while the
/// land enters, so the land itself (and lands entering with it) is not counted. Den of the Bugbear, Lair of the Hydra and
/// Hall of Storm Giants print it.
pub fn enters_tapped_if_two_or_more_other_lands(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if other_lands_you_control(g, entering) >= 2 {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control two or fewer other lands" (CR 614.1d): read on the battlefield as it is
/// while the land enters, so the land itself is not counted. Concealed Courtyard, Copperline Gorge and Darkslick
/// Shores print it.
pub fn enters_tapped_unless_two_or_fewer_other_lands(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let others = other_lands_you_control(g, entering);
    if others > 2 {
        entering.tapped = true;
    }
    Ok(())
}

/// "This land enters tapped unless you control two or more other lands" (CR 614.1d): read on the battlefield as it is
/// while the land enters, so the land itself is not counted. The slow lands: Shattered Sanctum, Stormcarved Coast and
/// Sundown Pass print it.
pub fn enters_tapped_unless_two_or_more_other_lands(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let others = other_lands_you_control(g, entering);
    if others < 2 {
        entering.tapped = true;
    }
    Ok(())
}

/// What a land that may be paid for with 2 life asks as it enters: "Pay 2 life" or "Enters tapped".
pub const PAY_TWO_LIFE_OR_TAPPED: Question =
    Question::new("EnterUntapped", "pay {cost}?", "If you don't, it enters tapped", "Pay {cost}", "Enters tapped").no_default();

/// What a land that may be paid for with 3 life asks as it enters: "Pay 3 life" or "Enters tapped".
pub const PAY_THREE_LIFE_OR_TAPPED: Question =
    Question::new("EnterUntapped", "pay {cost}?", "If you don't, it enters tapped", "Pay {cost}", "Enters tapped").no_default();

/// "Whenever [a player / you] tap[s] a land for mana" (CR 106.12, 106.12a, 605.1b): the land, the player who tapped it and
/// the mana its ability produced, for a triggered mana ability's matcher to narrow ("a basic land", "you") and hand to
/// `that_land_produced`. Only a land as it is once tapped: a land creature counts, a creature that isn't a land doesn't.
/// Lavaleaper, Mana Flare, Heartbeat of Spring, Dictate of Karametra, Zendikar Resurgent and Mirari's Wake trigger on it.
pub fn land_tapped_for_mana(q: &dyn Query, event: &Event) -> Option<(ObjectId, PlayerId, ManaTally)> {
    match *event {
        Event::TappedForMana { permanent, player, produced } if q.chars(permanent).is(CardType::Land) => Some((permanent, player, produced)),
        _ => None,
    }
}

/// The trigger of "that player adds one mana of any type that land produced": about the land, with the player who tapped
/// it (`data[0]`) and the types of mana it produced (`data[1]`, a bit for each `Mana` it produced), which the ability
/// resolving reads (`add_one_mana_of_any_type_that_land_produced`). None when it produced nothing, which triggers nothing
/// (CR 106.12a).
pub fn that_land_produced(land: ObjectId, player: PlayerId, produced: ManaTally) -> Option<Fired> {
    if produced.is_empty() {
        return None;
    }
    let types = Mana::ALL.iter().filter(|&&m| produced.get(m) > 0).fold(0u32, |types, &m| types | 1 << m as u32);
    Some(Fired { subject: Target::Object(land), data: [player.0 as u32, types] })
}

/// "That player adds one mana of any type that land produced" (CR 106.12a, 605.1b, 605.4a): a triggered mana ability's
/// effect, so it resolves at once as the land's mana ability does. The type is one of the land's own ability produced -
/// the mana a triggered mana ability added for it is not the land's (Zendikar Resurgent's ruling: the land's mana ability)
/// - and colorless is a type (CR 106.1b). With more than one, the player who adds it chooses (Mana Flare's, Vorinclex's
/// rulings). The mana is plain: no restriction or rider of the land's mana comes with it (Mana Flare's, Mirari's Wake's
/// rulings). Lavaleaper, Mana Flare, Heartbeat of Spring, Dictate of Karametra, Zendikar Resurgent, Mirari's Wake and
/// Vorinclex, Voice of Hunger print it.
pub fn add_one_mana_of_any_type_that_land_produced(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    let types: Vec<Mana> = Mana::ALL.into_iter().filter(|&m| item.data[1] & 1 << m as u32 != 0).collect();
    let mana = match types.as_slice() {
        [one] => *one,
        _ => {
            let source = item.object;
            types[crate::choose::one_of(g, player, source, types.iter().map(|&m| Label::ManaType(m)).collect())? as usize]
        }
    };
    g.add_mana(player, mana, 1);
    Ok(())
}

/// "Whenever a player taps a land for mana, that player adds one mana of any type that land produced" (CR 106.12a, 605.1b):
/// any player's land, as the triggered mana ability's matcher with `add_one_mana_of_any_type_that_land_produced`. Mana
/// Flare, Heartbeat of Spring, Dictate of Karametra and Zhur-Taa Ancient print it.
pub fn a_player_taps_a_land_for_mana(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    let (land, player, produced) = land_tapped_for_mana(q, event)?;
    that_land_produced(land, player, produced)
}

/// "Whenever you tap a [land type] for mana" (CR 106.12, 605.1b): a permanent of that land type (`TYPE`, `lands::sub(..)`),
/// read through the words of the ability's source (CR 612.2) and through the layers - a nonbasic land with the type counts,
/// a land named for it without the type doesn't - tapped for mana by the ability's controller. A delayed ability's
/// controller is the player who controlled the spell or ability that created it (CR 603.7d-e). About the permanent, the
/// player who tapped it in `data[0]` for `that_player_adds_an_additional`. Nissa, Who Shakes the World (Forest) and Pyre
/// Rhymer's Molten Tide (Mountain) print it.
pub fn you_tap_a_land_type_for_mana<const TYPE: u16>(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { player, .. } if player == asker.controller => a_player_taps_a_land_type_for_mana::<TYPE>(q, asker, event),
        _ => None,
    }
}

/// "Whenever a player taps a [land type] for mana" (CR 106.12, 605.1b): `you_tap_a_land_type_for_mana` for any player's
/// tap, the one who tapped it in `data[0]`. High Tide (Island) and Bubbling Muck (Swamp) print it.
pub fn a_player_taps_a_land_type_for_mana<const TYPE: u16>(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { permanent, player, .. } if q.chars(permanent).has_subtype(crate::lands::word(q, asker.source, TYPE)) => {
            Some(Fired { subject: Target::Object(permanent), data: [player.0 as u32, 0] })
        }
        _ => None,
    }
}

/// "[That player] add[s] an additional [mana]" (CR 605.1b, 605.4a): one mana of the kind `M` (`Mana::R as u8`), added by the
/// player who tapped the permanent (`data[0]`, from `you_tap_a_land_type_for_mana` or `a_player_taps_a_land_type_for_mana`),
/// whoever controls the ability - "that player" (High Tide), or "you" when only your tap triggers it (Nissa, Who Shakes the
/// World; Molten Tide). A triggered mana ability's effect, it resolves as the mana ability that triggered it does. A mana
/// symbol is no word (CR 612.2), so a text change leaves it.
pub fn that_player_adds_an_additional<const M: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(PlayerId(item.data[0] as u8), Mana::ALL[M as usize], 1);
    Ok(())
}

/// "Whenever you tap a `kind` for mana" (CR 605.1b): the trigger's event, about the permanent tapped.
fn you_tap_for_mana(q: &dyn Query, source: ObjectId, event: &Event, kind: CardType) -> Option<Fired> {
    match event {
        Event::TappedForMana { permanent, player, .. } if *player == q.controller(source) && q.chars(*permanent).is(kind) => {
            Some(Fired { subject: Target::Object(*permanent), data: [player.0 as u32, 0] })
        }
        _ => None,
    }
}

/// "Whenever you tap a creature for mana" (CR 605.1b), the player in `data[0]` for `that_player_adds_an_additional`. Badgermole
/// Cub prints it.
pub fn you_tap_a_creature_for_mana(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    you_tap_for_mana(q, source, event, CardType::Creature)
}

/// "Whenever you tap a land for mana" (CR 605.1b), as `you_tap_a_creature_for_mana`. Groundchuck & Dirtbag prints it.
pub fn you_tap_a_land_for_mana(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    you_tap_for_mana(q, source, event, CardType::Land)
}
