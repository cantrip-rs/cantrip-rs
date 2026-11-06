use cantrip_api::*;
use cantrip_api::select::{Choice, SelectFn, Verdict};
use cantrip_mechanics::{choose, lands, select, targets};
use cantrip_oracle as oracle;

/// Select a permanent card with the subtype named by the source's text (CR 110.4a, 612.2).
fn goblin_permanent(q: &dyn Query, choice: &Choice, subtype: u32) -> Verdict {
    let subtype = lands::printed(subtype as u16);
    Verdict::each(choice, |card| targets::is_permanent_card(&q.chars(card)) && q.chars(card).has_subtype(subtype))
}

/// You may put one Goblin permanent card from your hand onto the battlefield (CR 608.2d).
fn put_goblin_permanent_card_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hand = g.state().zone(Zone::hand(item.controller)).to_vec();
    let subtype = g.words(item.object).subtype(Subtype::Goblin);
    let rule = select::Selection::new(select::up_to(1))
        .with(select::part(&(goblin_permanent as SelectFn), lands::sub(subtype) as u32, ""));
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield, hand, rule)?;
    if let Some(&card) = chosen.first() {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    Ok(())
}

/// "{R}: Target Goblin gains protection from white until end of turn." (CR 115.1).
fn target_goblin_gains_protection_white_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Goblin])
}

/// "{R}: Target Goblin gains protection from white until end of turn." (CR 608.2, 611.2a).
fn target_goblin_gains_protection_white(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_5 = targets::object(g, item, 0);
    let kind_5_0_0 = EffectKind::Protection(ColorSet::of(g.words(item.object).color(Color::White)));
    instruction(
        g,
        |g| {
            if let Some(object) = objects_5 {
                g.add_effect(item.object, object, kind_5_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            put_goblin_permanent_card_hand,
            "{T}: You may put a Goblin permanent card from your hand onto the battlefield.",
        ),
        ability(
                cost("{R}"),
                target_goblin_gains_protection_white,
                "{R}: Target Goblin gains protection from white until end of turn.",
            )
            .targets(&[target(target_goblin_gains_protection_white_target_0)]),
    ],
    ..oracle::GOBLIN_WIZARD
};
