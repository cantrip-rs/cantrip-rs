//! "Choose a creature type" permanents (CR 205.3m, 614.12a): the type is chosen as the permanent enters and kept on it, and
//! its abilities say "of the chosen type". Banner of Kinship, Chronicle of Victory, Door of Destinies, Obelisk of Urd and Patchwork Banner print the lot.

use cantrip_api::*;

/// "As this enters, choose a creature type" (CR 614.12a): asked of the entering permanent's controller, any creature type,
/// and kept on the permanent as `chosen_subtype`. The type chosen, if one was.
pub fn choose_creature_type(g: &mut dyn Game, entering: &mut Entering) -> Result<Option<Subtype>, Suspend> {
    choose_type_among(g, entering, Subtype::CREATURE_TYPES.to_vec())
}

/// "As this enters, choose [one of these types]" (CR 614.12a): `choose_creature_type` among `choices` only, in the order
/// given (Eclipsed Realms' eight, read through its words).
pub fn choose_type_among(g: &mut dyn Game, entering: &mut Entering, choices: Vec<Subtype>) -> Result<Option<Subtype>, Suspend> {
    let ask = RequestKind::ChooseSubtype { source: entering.object, choices, purpose: WordUse::Chosen };
    let Answer::Subtype(chosen) = g.ask(entering.controller, ask)? else { return Ok(None) };
    entering.chosen_subtype = Some(chosen);
    Ok(Some(chosen))
}

/// The eight creature types of "choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk", words of the
/// card's text (CR 612: a text-changing effect changes what is offered).
const ELEMENTAL_ELF_FAERIE_GIANT_GOBLIN_KITHKIN_MERFOLK_TREEFOLK: [Subtype; 8] =
    [Subtype::Elemental, Subtype::Elf, Subtype::Faerie, Subtype::Giant, Subtype::Goblin, Subtype::Kithkin, Subtype::Merfolk, Subtype::Treefolk];

/// "As this enters, choose Elemental, Elf, Faerie, Giant, Goblin, Kithkin, Merfolk, or Treefolk" (CR 614.12a) as a card's
/// `enters`: one of the eight, asked of its controller as it enters and kept on it (CR 607.2a). Eclipsed Realms and
/// Dawn-Blessed Pennant print it.
pub fn enters_choosing_elemental_elf_faerie_giant_goblin_kithkin_merfolk_or_treefolk(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let words = g.words(entering.object);
    let choices = ELEMENTAL_ELF_FAERIE_GIANT_GOBLIN_KITHKIN_MERFOLK_TREEFOLK.iter().map(|&t| words.subtype(t)).collect();
    choose_type_among(g, entering, choices)?;
    Ok(())
}

/// "Choose a creature type" as a spell or ability resolves (CR 608.2d): asked of `player`, any creature type, and kept
/// nowhere - the effect uses it at once. Kindred Judgment, Harmonized Crescendo and Orcrist, Goblin-cleaver ask it.
pub fn creature_type_now(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<Subtype, Suspend> {
    let ask = RequestKind::ChooseSubtype { source, choices: Subtype::CREATURE_TYPES.to_vec(), purpose: WordUse::Chosen };
    // The core holds the answer to the question: one of the creature types offered (`validate`).
    let Answer::Subtype(chosen) = g.ask(player, ask)? else { unreachable!("the core answers a type choice with a type") };
    Ok(chosen)
}

/// `choose_creature_type` as a card's `enters`. Chronicle of Victory, Door of Destinies, Obelisk of Urd and Patchwork Banner print exactly this.
pub fn enters_choosing_a_creature_type(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    choose_creature_type(g, entering)?;
    Ok(())
}

/// "Creatures you control of the chosen type" (CR 205.3m): what a static ability of the permanent affects, a creature that
/// is the chosen type, under the permanent's controller. Banner of Kinship, Chronicle of Victory, Door of Destinies, Obelisk of Urd and Patchwork Banner
/// use it as `StaticDef::affects`.
pub fn creatures_you_control_of_chosen_type(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    s.chosen_subtype(q.linked_to(source)).is_some_and(|t| so_far.is(CardType::Creature) && so_far.has_subtype(t))
        && s.obj(affected).controller == s.obj(source).controller
}

/// "Creatures you control ... The same is true for creature spells you control and creature cards you own that aren't on
/// the battlefield" (CR 611.3a, 613.1d): the `affects` of a static ability declared `.everywhere()`, judged on what the layers
/// before made the object - a creature on the battlefield under the source's controller, a creature spell on the stack they
/// control, and a creature card they own anywhere else that is not the battlefield (a hand, a library, a graveyard, exile,
/// the stack under another player's control: a spell is a card there, CR 112.1, 108.2). A token or a copy of a card off
/// the battlefield is no card; a card outside the game the layers leave alone (CR 400.11c). Arcane Adaptation, Conspiracy
/// and Maskwood Nexus print it.
pub fn your_creatures_spells_and_cards(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    if !so_far.is(CardType::Creature) {
        return false;
    }
    let s = q.state();
    let o = s.obj(affected);
    let you = s.obj(source).controller;
    let your_card = o.owner == you && !o.token && !o.card_copy;
    match o.zone.kind() {
        ZoneKind::Battlefield => o.controller == you,
        ZoneKind::Stack => o.controller == you || your_card,
        _ => your_card,
    }
}

/// "Whenever you cast a spell of the chosen type" (CR 603.2): the spell's own subtypes on the stack (CR 205.3), the type
/// chosen for the ability that asks (`Query::linked_to`). Chronicle of Victory and Door of Destinies print it, asked with its
/// controller (`when_asked`).
pub fn you_cast_chosen_type(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == asker.controller => {
            let chosen = q.state().chosen_subtype(q.linked_to(asker))?;
            q.chars(*spell).has_subtype(chosen).then_some(Fired::default())
        }
        _ => None,
    }
}
