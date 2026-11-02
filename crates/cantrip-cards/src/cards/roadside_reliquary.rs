use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {C}." (CR 605.3a, 106.3).
fn add_c_mana(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::C, 1);
    Ok(())
}

/// "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment." (CR 608.2, 121.1).
fn draw_card_if_control_artifact(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if {
        let q: &dyn Query = g;
        q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.controller(id) == Some(item.controller).unwrap_or(PlayerId(u8::MAX))
                    && q.chars(id).is(CardType::Artifact)
            })
            .count() >= 1
    } {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.controller(id) == Some(item.controller).unwrap_or(PlayerId(u8::MAX))
                    && q.chars(id).is(CardType::Enchantment)
            })
            .count() >= 1
    } {
        g.draw(item.controller, 1);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            Adds {
                pool: ManaPool([0, 0, 0, 0, 0, 1]),
                ..add_by(add_c_mana)
            },
            "{T}: Add {C}.",
        ),
        ability(
            cost("{2}").and(SACRIFICE_THIS).and(TAP),
            draw_card_if_control_artifact,
            "{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment.",
        ),
    ],
    ..oracle::ROADSIDE_RELIQUARY
};
