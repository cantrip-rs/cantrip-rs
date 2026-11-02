use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever you draw a card" (CR 603.2).
fn when_draw_card(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Drew { player, .. } = event else { return None };
    let subject = Target::Player(*player);
    if asker.controller == *player {
        return Fired::about(subject);
    }
    None
}

/// "Whenever you draw a card, this creature gets +1/+0 until end of turn." (CR 608.2, 611.2a).
fn creature_gets_1_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(1, 0),
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// "Whenever this creature attacks, draw a card." (CR 608.2, 121.1).
fn draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "Discard a card: This creature gains hexproof until end of turn. Tap it." (CR 608.2, 611.2a, 701.26a).
fn creature_gains_hexproof_tap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_2 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_2_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Hexproof]));
    instruction(
        g,
        |g| {
            for &object in &objects_2 {
                g.add_effect(item.object, object, kind_2_0_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    let objects_3 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_3: Vec<ObjectId> = objects_3
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != true)
        .collect();
    g.tap_together(&turned_3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DREW, when_draw_card),
            creature_gets_1_0,
            "Whenever you draw a card, this creature gets +1/+0 until end of turn.",
        ),
        triggered(
            triggers::THIS_ATTACKS,
            draw_card,
            "Whenever this creature attacks, draw a card.",
        ),
        ability(
            discard_cards(1),
            creature_gains_hexproof_tap,
            "Discard a card: This creature gains hexproof until end of turn. Tap it.",
        ),
    ],
    ..oracle::DREAM_TRAWLER
};
