use cantrip_api::*;
use cantrip_mechanics::expend::{expend, EXPEND_4};
use cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE;
use cantrip_mechanics::{choose_colors, effects, library, permanents_with_subtype_you_control};
use cantrip_oracle as oracle;

/// "Add {R} or {G} for each Raccoon you control." Not a mana ability (CR 605.1b: it triggers on a step, not on a mana
/// ability), so it uses the stack; the Raccoons are counted as it resolves, Muerra among them while it is one, the subtype
/// read through the words of its text (CR 612.2), and a color is chosen for each mana, in any mix (CR 106.1b; its ruling).
/// The mana empties as the main phase ends (CR 106.4).
fn add_red_or_green_per_raccoon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let raccoons = {
        let q: &dyn Query = g;
        let raccoon = q.words(item.object).subtype(Subtype::Raccoon);
        permanents_with_subtype_you_control(q, item.controller, raccoon)
    };
    if raccoons == 0 {
        return Ok(());
    }
    for color in choose_colors(g, item.controller, item.object, ColorSet::of(Color::Red).with(Color::Green), raccoons as u32)? {
        g.add_mana(item.controller, color.mana(), 1);
    }
    Ok(())
}

/// "Exile the top two cards of your library. Until the end of your next turn, you may play those cards." (CR 701.18b, 611.2a)
fn exile_top_two_play_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_until_end_of_next_turn(g, item.controller, 2);
    Ok(())
}

/// "You gain 3 life." (CR 119.3)
fn you_gain_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_FIRST_MAIN_PHASE,
            add_red_or_green_per_raccoon,
            "At the beginning of your first main phase, add {R} or {G} for each Raccoon you control.",
        )
        .produces_mana(),
        triggered(
            EXPEND_4,
            you_gain_3_life,
            "Whenever you expend 4, you gain 3 life. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
        ),
        triggered(
            expend::<8>(),
            exile_top_two_play_them,
            "Whenever you expend 8, exile the top two cards of your library. Until the end of your next turn, you may play those cards.",
        ),
    ],
    ..oracle::MUERRA_TRASH_TACTICIAN
};
