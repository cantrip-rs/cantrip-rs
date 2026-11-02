use cantrip_api::*;
use cantrip_mechanics::{library, targets, tokens};
use cantrip_oracle as oracle;

/// "Creative Outburst deals 5 damage to any target. Look at the top five cards of your library. Put one of them into your hand and the rest on the bottom of your library in a random order. {U/R}{U/R}, Discard this card: Create a Treasure token." (CR 608.2, 120.3).
fn create_treasure_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let recipients_0 = t_0.clone();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 5u16))
        .collect();
    g.damage_together(&batch);
    g.end_sentence();
    library::look_at_top(
        g,
        item.controller,
        item.object,
        5,
        (1, 1),
        false,
        |_| true,
        library::Rest::BottomRandom,
    )?;
    Ok(())
}

/// "{U/R}{U/R}, Discard this card: Create a Treasure token." (CR 608.2, 111.1).
fn create_treasure_token_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_2 = Vec::new();
    let def_2_0 = g.card_id(&tokens::TREASURE);
    tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
    g.create_tokens_worded(&tokens_2, false, None, Words::NONE)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], create_treasure_token),
    abilities: &[
        ability(
                Cost::mana(ManaCost {
                        generic: 0,
                        x: 0,
                        symbols: [0, 0, 0, 0, 0, 0],
                        hybrid: [
                            Some((Mana::U, Mana::R)),
                            Some((Mana::U, Mana::R)),
                            None,
                            None,
                            None,
                            None,
                        ],
                        twobrid: [0, 0, 0, 0, 0],
                        phyrexian: [0, 0, 0, 0, 0],
                    })
                    .and(DISCARD_THIS),
                create_treasure_token_2,
                "{U/R}{U/R}, Discard this card: Create a Treasure token.",
            )
            .from(ZoneKind::Hand),
    ],
    ..oracle::CREATIVE_OUTBURST
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
