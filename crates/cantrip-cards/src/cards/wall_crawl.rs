use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_of_subtype};
use cantrip_mechanics::tokens::{self, SPIDER};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/1 green Spider creature token with reach, then you gain 1 life for each Spider you control." (CR 608.2c, 119.3):
/// the count is made after the token exists, so it is among the Spiders; every permanent you control with the Spider type as
/// it is now counts, one that is no creature too, and the life is one gain of all of them.
fn spider_then_life_for_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SPIDER, 1)?;
    // A sentence ends (CR 608.2c): what the token did is judged before the count is made.
    g.end_sentence();
    let spider = g.words(item.object).subtype(Subtype::Spider);
    let spiders = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(spider)).count();
    g.gain_life(item.controller,(spiders) as i32);
    Ok(())
}

/// "Spiders you control can't be blocked by creatures with defender." (CR 509.1b): of a possible block, the attacker a Spider its
/// controller - the source's - controls, the blocker a creature with defender, both as they are as blockers are declared.
fn spiders_evade_defenders(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    let spider = q.words(source).subtype(Subtype::Spider);
    !(q.controller(attacker) == q.controller(source) && q.chars(attacker).has_subtype(spider) && q.chars(blocker).has(Keyword::Defender))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        spider_then_life_for_each,
        "When this enchantment enters, create a 2/1 green Spider creature token with reach, then you gain 1 life for each Spider you control.",
    )],
    statics: &[static_ability(Layer::PtModify, your_of_subtype::<{ lands::sub(Subtype::Spider) }>, plus_pt::<1, 1>, "Spiders you control get +1/+1.")],
    block_rules: &[spiders_evade_defenders],
    ..oracle::WALL_CRAWL
};

pub static EXTRA: &[&CardDef] = &[&SPIDER];
