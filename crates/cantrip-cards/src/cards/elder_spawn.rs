use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature and it deals 6 damage to you." (CR 608.2, 120.3, 701.21a).
fn unless_sacrifice_island_sacrifice_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    const RULE: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Island)]).called("Island");
    let paid = effects::may_sacrifice(g, item.controller, item.object, RULE)?;
    g.end_sentence();
    if paid.is_empty() {
        g.sacrifice(item.controller, &[item.object]);
        g.end_sentence();
        g.damage_together(&[(item.object, Target::Player(item.controller), 6)]);
        g.end_sentence();
    }
    Ok(())
}

/// "This creature can't be blocked by red creatures" (CR 509.1b): red read through the source's own text (CR 612).
fn cant_be_blocked_by_red_creatures(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker
        || !(q.chars(blocker).is(CardType::Creature) && q.chars(blocker).colors.has(q.words(source).color(Color::Red)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            unless_sacrifice_island_sacrifice_creature,
            "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature and it deals 6 damage to you.",
        ),
    ],
    block_rules: &[cant_be_blocked_by_red_creatures],
    ..oracle::ELDER_SPAWN
};
