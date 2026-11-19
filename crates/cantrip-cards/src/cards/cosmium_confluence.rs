use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, choose, effects, library, select, targets};
use cantrip_oracle as oracle;

/// "A Cave card" (CR 205.3i): a land card with the subtype Cave, which may go unfound (CR 701.23b).
const CAVE_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Cave)]).called("Cave");

/// "Search your library for a Cave card, put it onto the battlefield tapped, then shuffle." (CR 701.23, 205.3i)
fn search_for_a_cave(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, CAVE_CARD, true)?;
    Ok(())
}

/// "Put three +1/+1 counters on a Cave you control. It becomes a 0/0 Elemental creature with haste. It's still a land." The Cave is
/// chosen as the mode resolves, not targeted (CR 608.2d), from the Caves its controller has then; with none, nothing happens (CR 608.2b).
/// Two sentences, so two instructions (CR 608.2c): the counters, then the permanent change with no duration (CR 611.2a), so it stays a
/// creature, a land and a Cave: the type and Elemental are added (CR 205.1b, "still a land"), the 0/0 set in layer 7b under the counters
/// of 7c. Chosen again by a later copy of this mode (CR 700.2d), it gets three more counters and the same change again.
fn counters_on_a_cave_that_becomes_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cave = g.words(item.object).subtype(Subtype::Cave);
    let caves: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(cave)).collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutCounters(CounterKind::PlusOne, 3), caves, select::cards(select::exactly(1)))?;
    let Some(&land) = chosen.first() else { return Ok(()) };
    g.add_counters(land, CounterKind::PlusOne, 3);
    g.end_sentence();
    let animation = Animation {
        power: 0,
        toughness: 0,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Elemental)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Haste]),
        ability: None,
    };
    animate(g, item.object, land, &animation, Duration::Indefinite);
    Ok(())
}

/// Choose three; you may choose the same mode more than once (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(search_for_a_cave, "Search your library for a Cave card, put it onto the battlefield tapped, then shuffle."),
    mode(
        counters_on_a_cave_that_becomes_a_creature,
        "Put three +1/+1 counters on a Cave you control. It becomes a 0/0 Elemental creature with haste. It's still a land.",
    ),
    mode(effects::destroy_first_target, "Destroy target enchantment.").targets(&[targets::ENCHANTMENT]),
])
.choose(3, 3)
.repeat();

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::COSMIUM_CONFLUENCE };
