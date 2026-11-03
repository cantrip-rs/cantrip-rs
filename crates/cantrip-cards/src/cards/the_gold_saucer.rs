use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "{2}, {T}: Flip a coin. If you win the flip, create a Treasure token." (CR 608.2, 111.1).
fn flip_coin_if_win_flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hoisted_4: Vec<ObjectId> = Vec::new();
    let flip_1 = g.flip_coin(item.controller);
    if flip_1 {
        let mut tokens_2 = Vec::new();
        let def_2_0 = g.card_id(&tokens::TREASURE);
        tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
        let created_2 = g.create_tokens_worded(&tokens_2, false, None, Words::NONE)?;
        g.end_sentence();
        hoisted_4.extend(created_2.clone());
    }
    Ok(())
}

/// "{3}, {T}, Sacrifice two artifacts: Draw a card." (CR 608.2, 121.1).
fn draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "{3}, {T}, Sacrifice two artifacts: Draw a card." (CR 608.2).
fn draw_card_2(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Artifact)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{2}").and(TAP),
            flip_coin_if_win_flip,
            "{2}, {T}: Flip a coin. If you win the flip, create a Treasure token.",
        ),
        ability(
            cost("{3}").and(TAP).and(sacrifice(2, draw_card_2)),
            draw_card,
            "{3}, {T}, Sacrifice two artifacts: Draw a card.",
        ),
    ],
    ..oracle::THE_GOLD_SAUCER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
