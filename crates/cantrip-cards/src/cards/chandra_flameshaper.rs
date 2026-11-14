use cantrip_api::*;
use cantrip_mechanics::copy::{self, SACRIFICE_THIS_TOKEN_EXCEPTION};
use cantrip_mechanics::{damage_divided, library, targets};
use cantrip_oracle as oracle;

/// +2: "Add {R}{R}{R}. Exile the top three cards of your library. Choose one. You may play that card this turn." Each sentence
/// ends where the text's does (CR 608.2c); the three cards are exiled together, one is chosen from those that reached exile, and
/// only that card can be played this turn (a land under the ordinary land-play rules, CR 305.1).
fn add_mana_and_exile_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 3);
    g.end_sentence();
    let grant = PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    library::exile_top_choose_one_may_play(g, item.controller, item.object, 3, grant)?;
    Ok(())
}

/// −4: "Chandra deals 8 damage divided as you choose among any number of target creatures and/or planeswalkers" (CR 601.2d): any
/// number includes zero (CR 107.1c); each chosen target takes its own share, and an illegal target is left out.
fn divided_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_divided(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), add_mana_and_exile_three, "+2: Add {R}{R}{R}. Exile the top three cards of your library. Choose one. You may play that card this turn.")
            .produces_mana(),
        ability(
            loyalty(1),
            copy::create_token_copy_with_haste_and_sacrifice,
            "+1: Create a token that's a copy of target creature you control, except it has haste and \"At the beginning of the end step, sacrifice this token.\"",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(loyalty(-4), divided_damage, "\u{2212}4: Chandra deals 8 damage divided as you choose among any number of target creatures and/or planeswalkers.")
            .targets(&[targets::CREATURE_OR_PLANESWALKER.between(0, TargetSpec::COUNTED)])
            .divide(Divide { total: 8, reads: None }),
    ],
    ..oracle::CHANDRA_FLAMESHAPER
};

pub static EXTRA: &[&CardDef] = &[&SACRIFICE_THIS_TOKEN_EXCEPTION];
