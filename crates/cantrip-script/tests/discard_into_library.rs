//! A trigger on its own card being discarded functions in every zone the discarded card can wind up in (CR 113.6k,
//! 702.29c), a library included: Wheel of Sun and Moon reveals a card bound for its owner's graveyard and puts it on the
//! bottom of their library, and "when you discard this card" (Titanbones, Towering Heart) and "when you cycle this card"
//! (the cycling decrees) trigger from there. Library of Leng puts a card discarded by an effect on top of its owner's
//! library unrevealed: its characteristics are undefined (CR 701.9c), its abilities among them (CR 109.3), and it triggers
//! on nothing. A card discarded to pay a cost that specifies a characteristic of it can't pay it from there (CR 701.9c,
//! 118.12), so no such replacement is offered for it. No printed card in the engine has such a trigger or puts a discarded
//! card into a library yet, so test-only ones do (`cantrip_script::run_on`).

#[path = "support/discard_library.rs"]
mod discard_library;
use discard_library::play;

/// Faithless Looting discards Probe Bones and an Island, and the Leng puts both on top of the library unrevealed: the
/// Bones' characteristics are undefined there (CR 701.9c), and its "when you discard this card" does not trigger. (Its
/// revealed twin below triggers.)
#[test]
fn a_card_discarded_unrevealed_into_a_library_does_not_trigger() {
    play("discard-trigger-not-from-a-hidden-library", &script("Probe Leng", "? Stack.Count = 0", 20));
}

/// The same discard, with the card revealed as it is put on top of the library: it triggers from there (CR 113.6k), 3 life.
/// A second Probe Bones already in the library is not the card discarded and does not trigger, nor does the Island.
#[test]
fn a_card_discarded_revealed_into_a_library_triggers_from_there() {
    play("discard-trigger-from-a-library", &script("Probe Revealing Leng", "? Stack.Count = 1", 23));
}

/// Faithless Looting under `leng`, discarding Probe Bones and an Island into the library: the stack checked after the
/// discard, and Bot1's life at the end.
fn script(leng: &str, stack: &str, life: u32) -> String {
    format!(
        "
[Bot1 Deck]
{leng}
Mountain
Faithless Looting
Probe Bones
Island
Island
Island
Island
Island
Probe Bones
Island x12

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mountain.Play
Bot1.Battlefield.Mountain.Activate
Bot1.Hand.Faithless Looting.Cast
Bot1.Pass
Bot1.Choose(Probe Bones; Island)
{stack}
? Bot1.Library.Count = 15
Bot1.Pass
Bot2.Pass

[Expect]
Bot1.Life = {life}
Bot1.Library.Count = 15
Bot1.Graveyard = Faithless Looting
Bot1.Hand.Count = 4
"
    )
}

/// Faithless Looting discards Probe Shuffling Bones, which is revealed and shuffled into the library instead of reaching
/// the graveyard: its trigger functions in the library it was shuffled into, and 3 life.
#[test]
fn a_card_discarded_and_shuffled_into_a_library_triggers_from_there() {
    play(
        "discard-trigger-from-a-shuffled-library",
        "
[Bot1 Deck]
Mountain
Faithless Looting
Probe Shuffling Bones
Island
Island
Island
Island
Island
Island
Island x12

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mountain.Play
Bot1.Battlefield.Mountain.Activate
Bot1.Hand.Faithless Looting.Cast
Bot1.Pass
Bot1.Choose(Probe Shuffling Bones; Island)
? Stack.Count = 1
Bot1.Pass
Bot2.Pass

[Expect]
Bot1.Life = 23
Bot1.Library.Count = 13
Bot1.Graveyard = Faithless Looting; Island
",
    );
}

/// Aclazotz's "whenever an opponent discards a land card" reads the discarded card (CR 701.9a), which the Leng put on top of
/// its owner's library: unrevealed it is undefined and no land card (CR 701.9c), so no Bat; revealed, two lands are two Bats.
#[test]
fn an_opponents_land_discarded_unrevealed_into_a_library_is_no_land_card_to_aclazotz() {
    play("aclazotz-hidden-discard", &aclazotz("Probe Leng", 0));
}

#[test]
fn an_opponents_land_discarded_revealed_into_a_library_is_a_land_card_to_aclazotz() {
    play("aclazotz-revealed-discard", &aclazotz("Probe Revealing Leng", 2));
}

/// Bot1 puts the Leng and Aclazotz onto the battlefield; Bot2 loots away two Islands under it.
fn aclazotz(leng: &str, bats: u32) -> String {
    format!(
        "
[Bot1 Deck]
{leng}
Black Lotus
Mox Jet
Mox Ruby
Aclazotz, Deepest Betrayal
Island x15

[Bot2 Deck]
Mountain
Faithless Looting
Island
Island
Island
Island
Island
Island x13

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Aclazotz, Deepest Betrayal.Cast
Bot1.Pass
Bot2.Pass
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
Bot2.Battlefield.Mountain.Activate
Bot2.Hand.Faithless Looting.Cast
Bot2.Pass
Bot2.Choose(Island; Island)
Bot2.Pass
Bot1.Pass

[Expect]
Bot1.Battlefield.Count = {count}
Bot2.Library.Count = 12
Bot2.Graveyard = Faithless Looting
",
        count = 4 + bats
    )
}

/// Seasoned Pyromancer discards a Lightning Bolt and an Island, which the Leng puts on top of the library: unrevealed, the
/// Bolt is undefined and no nonland card (CR 701.9c), so no Elemental; revealed, it is one, and one Elemental.
#[test]
fn a_card_discarded_unrevealed_into_a_library_is_no_nonland_card_to_seasoned_pyromancer() {
    play("pyromancer-hidden-discard", &pyromancer("Probe Leng", 0));
}

#[test]
fn a_card_discarded_revealed_into_a_library_is_a_nonland_card_to_seasoned_pyromancer() {
    play("pyromancer-revealed-discard", &pyromancer("Probe Revealing Leng", 1));
}

fn pyromancer(leng: &str, elementals: u32) -> String {
    format!(
        "
[Bot1 Deck]
{leng}
Black Lotus
Seasoned Pyromancer
Lightning Bolt
Island
Island
Island
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Seasoned Pyromancer.Cast
Bot1.Pass
Bot2.Pass
Bot1.Choose(Lightning Bolt; Island)
Bot1.Pass
Bot2.Pass

[Expect]
Bot1.Battlefield.Count = {count}
Bot1.Library.Count = 15
",
        count = 2 + elementals
    )
}

/// A pair of scripts under the hidden and the revealing Leng: `make(leng, revealed)`.
fn both(name: &str, make: fn(&str, bool) -> String) {
    play(&format!("{name}-hidden"), &make("Probe Leng", false));
    play(&format!("{name}-revealed"), &make("Probe Revealing Leng", true));
}

/// Ledger Shredder connives on Bot1's second spell of turn 3 and discards a Lightning Bolt into the library: unrevealed it is
/// undefined and no nonland card (CR 701.9c), so no counter (CR 701.50a); revealed, one.
#[test]
fn connive_counts_no_nonland_card_discarded_unrevealed_into_a_library() {
    both("connive", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Black Lotus
Ledger Shredder
Ornithopter
Ornithopter
Lightning Bolt
Island
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Ledger Shredder.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main1]
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Choose(Lightning Bolt)
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main2]

[Expect]
Bot1.Battlefield.Ledger Shredder.Counters.PlusOnePlusOne = {counters}
Bot1.Graveyard = Black Lotus
",
            counters = if revealed { "1" } else { "(absent)" }
        )
    });
}

/// Seasoned Cryomancer discards a Lightning Bolt and an Island into the library: unrevealed, no nonland card was discarded
/// this way (CR 701.9c) and the reflexive ability does not trigger; revealed, it taps and stuns the Ornithopter.
#[test]
fn seasoned_cryomancer_sees_no_nonland_card_discarded_unrevealed_into_a_library() {
    both("cryomancer", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Ornithopter
Black Lotus
Mox Sapphire
Seasoned Cryomancer
Island
Island
Lightning Bolt
Island
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Seasoned Cryomancer.Cast
Bot1.Pass
Bot2.Pass
Bot1.Choose(Lightning Bolt; Island)
{target}
[Bot1.Main2]

[Expect]
Bot1.Battlefield.Ornithopter.Tapped = {revealed}
Bot1.Graveyard = Black Lotus
",
            // Unrevealed, the reflexive ability's "up to that many" targets are never asked for.
            target = if revealed { "Bot1.Target(Ornithopter)" } else { "! Bot1.Target" }
        )
    });
}

/// Glacial Dragonhunt discards a Lightning Bolt into the library: unrevealed it is no nonland card (CR 701.9c) and the
/// Ornithopter lives; revealed, the reflexive ability deals it 3 damage.
#[test]
fn glacial_dragonhunt_sees_no_nonland_card_discarded_unrevealed_into_a_library() {
    both("dragonhunt", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Mox Sapphire
Mox Ruby
Ornithopter
Glacial Dragonhunt
Lightning Bolt
Island
Island x15

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Glacial Dragonhunt.Cast
Bot1.Pass
Bot2.Pass
Bot1.Choose(Lightning Bolt)
{target}
[Bot1.Main2]

[Expect]
Bot1.Battlefield.Count = {count}
",
            // Its only legal target, the Ornithopter, is chosen for it.
            target = if revealed { "" } else { "? Stack.Count = 0" },
            count = if revealed { 3 } else { 4 }
        )
    });
}

/// Garruk's −3 has Bot2 discard a Lightning Bolt and a Counterspell into the library: unrevealed they are no nonland cards
/// (CR 701.9c), so Bot2 "didn't discard two nonland cards this way" and Bot1 draws; revealed, Bot1 draws nothing.
#[test]
fn garruk_counts_no_nonland_card_discarded_unrevealed_into_a_library() {
    both("garruk", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Black Lotus
Mox Jet
Mox Jet
Garruk, Veiled Butcher
Forest
Forest
Forest x15

[Bot2 Deck]
Lightning Bolt
Counterspell
Island x18

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Garruk, Veiled Butcher.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Garruk, Veiled Butcher.Activate(Ability: Each opponent discards)
Bot1.Pass
Bot2.Pass
Bot2.Choose(Lightning Bolt; Counterspell)
[Bot1.Main2]

[Expect]
Bot1.Hand.Count = {hand}
Bot2.Library.Count = 15
",
            hand = if revealed { 2 } else { 3 }
        )
    });
}

/// Occult Epiphany (X = 3) discards an Ornithopter and two Islands into the library: unrevealed they are undefined and add
/// no card type (CR 701.9c), so no Spirit; revealed, artifact, creature and land make three.
#[test]
fn occult_epiphany_counts_no_type_of_a_card_discarded_unrevealed_into_a_library() {
    both("epiphany", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Black Lotus
Mox Sapphire
Occult Epiphany
Island
Island
Island
Island
Ornithopter
Island
Island x10

[Bot2 Deck]
Swamp x15

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Occult Epiphany.Cast(X: 3)
Bot1.Choose(Ornithopter; Island; Island)
Bot1.Pass
[Bot1.Main2]

[Expect]
Bot1.Battlefield.Count = {count}
",
            count = if revealed { 5 } else { 2 }
        )
    });
}

/// Ill-Timed Explosion discards a Lightning Bolt and a Counterspell into the library: unrevealed they have no mana value (CR
/// 701.9c), so X is 0 and the Ornithopter (0/2) lives; revealed, X is 2 and it dies.
#[test]
fn ill_timed_explosion_reads_no_mana_value_of_a_card_discarded_unrevealed_into_a_library() {
    both("explosion", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Mox Sapphire
Mox Ruby
Mox Jet
Mox Pearl
Ill-Timed Explosion
Ornithopter
Lightning Bolt
Counterspell
Plains x10

[Bot2 Deck]
Plains x15

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Ill-Timed Explosion.Cast
Bot1.Accept
Bot1.Choose(Lightning Bolt; Counterspell)
? Stack.Count = 1
Bot1.Pass
Bot2.Pass
[Bot1.Main2]

[Expect]
Bot1.Battlefield.Count = {count}
Bot1.Library.Count = 12
",
            count = if revealed { 5 } else { 6 }
        )
    });
}

/// Recruit (The Queen of Dale, on Bot2's Lightning Bolt) discards an Ornithopter into the library: unrevealed it is no
/// nonland card (CR 701.9c), so no Human Soldier (CR 701.70a); revealed, one.
#[test]
fn recruit_sees_no_nonland_card_discarded_unrevealed_into_a_library() {
    both("recruit", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Plains
Mox Pearl
The Queen of Dale
Ornithopter
Island
Island
Island x20

[Bot2 Deck]
Mountain
Lightning Bolt
Mountain x5
Mountain x20

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Plains.Play
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Battlefield.Plains.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.The Queen of Dale.Cast
Bot1.Pass
[T2 Bot2.Main1]
Bot2.Hand.Mountain.Play
Bot2.Battlefield.Mountain.Activate
Bot2.Hand.Lightning Bolt.Cast(Bot1)
Bot2.Pass
Bot1.Choose(Ornithopter)
Bot2.Pass
Bot1.Pass
[T2 Bot2.Main2]

[Expect]
Bot1.Life = 17
Bot1.Battlefield.Count = {count}
Bot1.Graveyard.Count = 0
",
            count = if revealed { 5 } else { 4 }
        )
    });
}

/// Hollow Marauder has Bot2 discard a Force of Will (mana value 5) into the library: unrevealed it is undefined and has no
/// mana value (CR 701.9c), so Bot2 "didn't discard a card with mana value 4 or greater" and Bot1 draws; revealed, no draw.
#[test]
fn hollow_marauder_reads_no_mana_value_of_a_card_discarded_unrevealed_into_a_library() {
    both("marauder", |leng, revealed| {
        format!(
            "
[Bot1 Deck]
{leng}
Black Lotus
Mox Jet
Mox Ruby
Mox Emerald
Mox Pearl
Hollow Marauder
Swamp x13

[Bot2 Deck]
Force of Will
Island x19

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Ruby.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Hollow Marauder.Cast
Bot1.Pass
Bot2.Pass
Bot1.Target(Bot2)
Bot1.Pass
Bot2.Pass
Bot2.Choose(Force of Will)
[Bot1.Main2]

[Expect]
Bot1.Hand.Count = {hand}
Bot2.Library.Count = 14
",
            hand = if revealed { 0 } else { 1 }
        )
    });
}

// A card discarded to pay a cost that specifies a characteristic of it - "unless you discard a nonland card", "you may discard
// a land card instead", "Discard a creature card:" (CR 118.12) - can't pay it from a hidden zone it was put into unrevealed (CR
// 701.9c): the Leng's library is not offered for it, and it goes to the graveyard. A discard that specifies nothing ("discard
// two cards", "Discard a card:") still goes to the library.

/// Bandit's Talent under `leng`: Bot2 pays the "unless" with a Lightning Bolt, which the Leng can't put on top of the library
/// unrevealed; or discards two cards, the Bolt among them, which specify no characteristic, and both go there.
fn bandits_talent(leng: &str, answer: &str, graveyard: &str, library: u32) -> String {
    format!(
        "
[Bot1 Deck]
{leng}
Mox Jet
Mox Ruby
Bandit's Talent
Island x16

[Bot2 Deck]
Lightning Bolt
Island x19

[Actions]
Bot1.Hand.{leng}.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Bandit's Talent.Cast
{answer}
[Bot1.Main2]

[Expect]
{graveyard}
Bot2.Library.Count = {library}
"
    )
}

#[test]
fn bandits_talent_nonland_card_paying_the_unless_is_not_put_into_a_library() {
    play("bandits-talent-paid-nonland", &bandits_talent("Probe Leng", "Bot2.Modes(Discard a nonland card)\nBot2.Choose(Lightning Bolt)", "Bot2.Graveyard = Lightning Bolt", 13));
}

#[test]
fn bandits_talent_two_cards_discarded_may_be_put_into_a_library() {
    play("bandits-talent-two-cards", &bandits_talent("Probe Leng", "Bot2.Modes(Discard two cards)\nBot2.Choose(Lightning Bolt; Island)", "Bot2.Graveyard.Count = 0", 15));
}

/// A Leng that reveals the card as it puts it on top of the library leaves its characteristics known (CR 701.9c), so the Bolt
/// paying the "unless" may go there.
#[test]
fn bandits_talent_nonland_card_paying_the_unless_may_be_revealed_into_a_library() {
    play(
        "bandits-talent-paid-nonland-revealed",
        &bandits_talent("Probe Revealing Leng", "Bot2.Modes(Discard a nonland card)\nBot2.Choose(Lightning Bolt)", "Bot2.Graveyard.Count = 0", 14),
    );
}

/// In one simultaneous discard (CR 101.4, 701.9a), Bot2's Lightning Bolt pays the "unless" and goes to the graveyard while
/// Bot3's two Forests, paying nothing that names a characteristic, go on top of the library.
#[test]
fn bandits_talent_judges_each_card_of_one_discard_by_the_cost_it_pays() {
    play(
        "bandits-talent-mixed",
        "
[Bot1 Deck]
Probe Leng
Mox Jet
Mox Ruby
Bandit's Talent
Island x16

[Bot2 Deck]
Lightning Bolt
Island x19

[Bot3 Deck]
Forest x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Bandit's Talent.Cast
Bot2.Modes(Discard a nonland card)
Bot2.Choose(Lightning Bolt)
Bot3.Choose(Forest; Forest)
[Bot1.Main2]

[Expect]
Bot2.Graveyard = Lightning Bolt
Bot2.Library.Count = 13
Bot3.Graveyard.Count = 0
Bot3.Library.Count = 15
",
    );
}

/// Library of Leng's choice, under Bot2's own Leng: the two cards discarded for Bandit's Talent may each be put on top of the
/// library - the question is asked, once for each, and taken.
#[test]
fn bandits_talent_two_cards_discarded_are_offered_to_an_optional_leng() {
    play(
        "bandits-talent-optional-leng",
        "
[Bot1 Deck]
Mox Jet
Mox Ruby
Bandit's Talent
Island x17

[Bot2 Deck]
Probe Optional Leng
Lightning Bolt
Island x18

[Actions]
[T2 Bot2.Main1]
Bot2.Hand.Probe Optional Leng.Cast
Bot2.Pass
Bot1.Pass
[T3 Bot1.Main1]
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Bandit's Talent.Cast
Bot2.Modes(Discard two cards)
Bot2.Choose(Island; Island)
Bot2.Accept
Bot2.Accept
[T3 Bot1.Main2]

[Expect]
Bot2.Graveyard.Count = 0
Bot2.Library.Count = 14
",
    );
}

/// Steamcore Scholar's "unless you discard an instant or sorcery card": the Lightning Bolt goes to the graveyard, not the library.
#[test]
fn steamcore_scholar_instant_paying_the_unless_is_not_put_into_a_library() {
    play(
        "steamcore-scholar-paid-instant",
        "
[Bot1 Deck]
Probe Leng
Mox Sapphire
Mox Ruby
Mox Jet
Steamcore Scholar
Lightning Bolt
Island
Plains x13

[Bot2 Deck]
Plains x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Steamcore Scholar.Cast
Bot1.Modes(Discard an instant)
Bot1.Choose(Lightning Bolt)
[Bot1.Main2]

[Expect]
Bot1.Graveyard = Lightning Bolt
Bot1.Library.Count = 11
",
    );
}

/// Winternight Stories' "unless you discard a creature card": the Ornithopter goes to the graveyard, not the library.
#[test]
fn winternight_stories_creature_paying_the_unless_is_not_put_into_a_library() {
    play(
        "winternight-stories-paid-creature",
        "
[Bot1 Deck]
Probe Leng
Mox Sapphire
Mox Jet
Mox Pearl
Winternight Stories
Ornithopter
Plains
Plains
Llanowar Elves
Island
Plains x10

[Bot2 Deck]
Plains x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Winternight Stories.Cast
Bot1.Modes(Discard a creature card)
Bot1.Choose(Ornithopter)
[Bot1.Main2]

[Expect]
Bot1.Graveyard = Winternight Stories; Ornithopter
Bot1.Library.Count = 10
",
    );
}

/// Mox Diamond's "you may discard a land card instead": the Forest goes to the graveyard, not the library, and the Mox enters.
#[test]
fn mox_diamond_land_is_not_put_into_a_library() {
    play(
        "mox-diamond-paid-land",
        "
[Bot1 Deck]
Probe Leng
Mox Diamond
Island
Forest
Island x16

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Island.Play
Bot1.Hand.Mox Diamond.Cast
Bot1.Accept
Bot1.Choose(Forest)
[T1 Bot1.Main2]

[Expect]
Bot1.Battlefield = Probe Leng; Island; Mox Diamond
Bot1.Graveyard = Forest
Bot1.Library.Count = 13
",
    );
}

/// Loamcrafter Faun's "you may discard one or more land cards": the Island goes to the graveyard, not the library, and the
/// reflexive ability returns the Black Lotus.
#[test]
fn loamcrafter_faun_land_is_not_put_into_a_library() {
    play(
        "loamcrafter-faun-paid-land",
        "
[Bot1 Deck]
Probe Leng
Forest
Lotus Petal
Black Lotus
Lightning Bolt
Loamcrafter Faun
Island
Island x13

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Forest.Play
Bot1.Hand.Lotus Petal.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Lightning Bolt.Cast(Bot2)
Bot1.Battlefield.Lotus Petal.Activate(Color: Green)
Bot1.Battlefield.Forest.Activate
Bot1.Hand.Loamcrafter Faun.Cast
Bot1.Accept
Bot1.Choose(Island)
Bot1.Target(Black Lotus)

[Expect]
Bot1.Hand = Black Lotus
Bot1.Graveyard = Island; Lightning Bolt; Lotus Petal
Bot1.Library.Count = 13
",
    );
}

/// Survival of the Fittest's "Discard a creature card" cost (CR 118.3, 602.2b): the Birds of Paradise goes to the graveyard, not
/// the library.
#[test]
fn a_cost_naming_a_creature_card_does_not_put_it_into_a_library() {
    play(
        "survival-paid-creature",
        "
[Bot1 Deck]
Probe Leng
Forest
Black Lotus
Survival of the Fittest
Birds of Paradise
Lightning Bolt
Forest
Elvish Mystic
Noble Hierarch
Forest x11

[Bot2 Deck]
Swamp x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Forest.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Hand.Survival of the Fittest.Cast
Bot1.Battlefield.Survival of the Fittest.Activate
Bot1.Choose(Pay mana)
Bot1.Choose(Birds of Paradise)
Bot1.Search(Noble Hierarch)
[Bot1.Main1]

[Expect]
Bot1.Graveyard = Birds of Paradise; Black Lotus
Bot1.Hand = Noble Hierarch; Lightning Bolt; Forest
",
    );
}

/// Psychic Frog's "Discard a card" cost names no characteristic: the Lightning Bolt goes on top of the library.
#[test]
fn a_cost_of_any_card_may_put_it_into_a_library() {
    play(
        "frog-paid-any-card",
        "
[Bot1 Deck]
Probe Leng
Black Lotus
Psychic Frog
Lightning Bolt
Island
Island
Island
Island x13

[Bot2 Deck]
Swamp x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Island.Play
Bot1.Battlefield.Island.Activate
Bot1.Hand.Psychic Frog.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Psychic Frog.Activate
Bot1.Choose(Lightning Bolt)
Bot1.Pass
Bot2.Pass
[Bot1.Main2]

[Expect]
Bot1.Graveyard = Black Lotus
Bot1.Library.Count = 14
Bot1.Battlefield.Psychic Frog.Counters.PlusOnePlusOne = 1
",
    );
}

/// Collective Brutality retraced (Wrenn and Six's emblem) and escalated once: retrace's "discard a land card" (CR 702.81a)
/// names a characteristic and escalate's "discard a card" (CR 702.120a) does not, so of the two cards discarded together the
/// Forest goes to the graveyard and the Chain Lightning on top of the library.
#[test]
fn retrace_land_is_not_put_into_a_library_but_escalates_card_is() {
    play(
        "retrace-and-escalate-under-leng",
        "
[Bot1 Deck]
Probe Leng
Mountain
Forest
Wrenn and Six
Collective Brutality
Swamp
Forest
Chain Lightning
Island x20

[Bot2 Deck]
Lightning Bolt
Island x20

[Actions]
Bot1.Hand.Mountain.Play
[T3 Bot1.Main1]
Bot1.Hand.Forest.Play
Bot1.Battlefield.Mountain.Activate
Bot1.Battlefield.Forest.Activate
Bot1.Hand.Wrenn and Six.Cast
Bot1.Battlefield.Wrenn and Six.Activate(Ability: Return)
Bot1.Battlefield.Wrenn and Six.Activate(Ability: Return)
Bot1.Battlefield.Wrenn and Six.Activate(Ability: Return)
Bot1.Battlefield.Wrenn and Six.Activate(Ability: Return)
Bot1.Discard(Collective Brutality)
[T11 Bot1.Main1]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Swamp.Play
Bot1.Battlefield.Wrenn and Six.Activate(Ability: emblem)
Bot1.Pass
Bot1.Battlefield.Swamp.Activate
Bot1.Battlefield.Mountain.Activate
Bot1.Graveyard.Collective Brutality.Cast
Bot1.Modes(1; 3)
Bot1.Target(Bot2)
Bot1.Target(Bot2)
Bot1.Choose(Pay mana)
Bot1.Choose(Pay mana)
Bot1.Choose(Cost: 1)
Bot1.Choose(Forest)
Bot1.Choose(Chain Lightning)
Bot1.Choose(Lightning Bolt)

[Expect]
Bot2.Life = 18
Bot1.Graveyard.Count = 3
Bot1.Library.Count = 17
Bot1.Hand = Island; Island; Island; Island
",
    );
}

/// Visage of Dread has Bot2 reveal their hand and discard the Probe Bones chosen from it: the card stays revealed through the
/// discard (CR 701.20a), so the Leng puts it on top of the library seen by all, its characteristics known (CR 701.9c applies
/// only without a reveal), and its "when you discard this card" triggers from there (CR 113.6k): Bot2 gains 3.
#[test]
fn a_card_discarded_from_a_revealed_hand_into_a_library_is_known() {
    play(
        "revealed-hand-discard-under-leng",
        "
[Bot1 Deck]
Probe Leng
Mox Jet
Mox Ruby
Visage of Dread
Island x16

[Bot2 Deck]
Probe Bones
Island x19

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Visage of Dread.Cast
Bot1.Pass
Bot2.Pass
Bot1.Pass
Bot2.Pass
Bot1.Choose(Probe Bones)
? Stack.Count = 1
Bot1.Pass
Bot2.Pass
[Bot1.Main2]

[Expect]
Bot2.Life = 23
Bot2.Library.Count = 14
Bot2.Graveyard.Count = 0
",
    );
}

