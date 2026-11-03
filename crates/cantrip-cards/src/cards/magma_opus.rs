use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, damage_divided};
use cantrip_oracle as oracle;

/// "Magma Opus deals 4 damage divided as you choose among any number of targets. Tap two target permanents. Create a 4/4 blue and red Elemental creature token. Draw two cards. {U/R}{U/R}, Discard this card: Create a Treasure token." (CR 608.2, 121.1, 111.1).
fn create_treasure_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_1 = announced(g, item, 1, true);
    damage_divided(g, item);
    g.end_sentence();
    let objects_1 = t_1.iter().filter_map(|t| t.object()).collect::<Vec<ObjectId>>();
    g.tap_together(&objects_1);
    g.end_sentence();
    let mut tokens_2 = Vec::new();
    let def_2_0 = g.card_id(&tokens::ELEMENTAL);
    tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
    let words_2 = g.words(item.object);
    g.create_tokens_worded(&tokens_2, false, None, words_2)?;
    g.end_sentence();
    g.draw(item.controller, 2);
    Ok(())
}

/// "{U/R}{U/R}, Discard this card: Create a Treasure token." (CR 608.2, 111.1).
fn create_treasure_token_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_5 = Vec::new();
    let def_5_0 = g.card_id(&tokens::TREASURE);
    tokens_5.extend(std::iter::repeat((def_5_0, item.controller)).take(1));
    g.create_tokens_worded(&tokens_5, false, None, Words::NONE)?;
    Ok(())
}

/// "Magma Opus deals 4 damage divided as you choose among any number of targets. Tap two target permanents. Create a 4/4 blue and red Elemental creature token. Draw two cards. {U/R}{U/R}, Discard this card: Create a Treasure token." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::ANY.between(0, TargetSpec::ANY_NUMBER),
            targets::PERMANENT.between(2, 2),
        ],
        create_treasure_token,
    ),
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
    divide: Some(Divide { total: 4, of_x: None }),
    ..oracle::MAGMA_OPUS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEMENTAL, &tokens::TREASURE];
