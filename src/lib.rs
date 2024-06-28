#![no_std]
extern crate alloc;

use alloc::collections::BTreeMap;
use gstd::{ActorId, exec, msg, prelude::*};
use pebbles_game_io::*;


static mut PEBBLES_GAMES: Option<BTreeMap<ActorId, GameState>> = None;

static mut PEBBLES_INIT: Option<PebblesInit> = None;


pub fn initialize_pebbles_init(init_value: PebblesInit) {
    unsafe {
        PEBBLES_INIT = Some(init_value);
    }
}

pub fn get_pebbles_difficulty() -> Option<DifficultyLevel> {
    unsafe {
        PEBBLES_INIT.as_ref().map(|init| init.difficulty.clone())
    }
}

pub fn get_pebbles_count() -> Option<u32> {
    unsafe {
        PEBBLES_INIT.as_ref().map(|init| init.pebbles_count.clone())
    }
}

pub fn get_max_pebbles_per_turn() -> Option<u32> {
    unsafe {
        PEBBLES_INIT.as_ref().map(|init| init.max_pebbles_per_turn.clone())
    }
}


fn initialize_pebbles_games() {
    unsafe {
        PEBBLES_GAMES = Some(BTreeMap::new());
    }
}

fn remove_game(actor_id: ActorId) {
    unsafe {
        if let Some(ref mut pebbles_games) = PEBBLES_GAMES {
            pebbles_games.remove(&actor_id);
        }
    }
}

fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

fn get_random_num_1_to_k(k: u32) -> u32 {
    // Ensure k is at least 1 to avoid division by zero
    assert!(k >= 1, "k must be at least 1");

    let random_value = get_random_u32();
    // Calculate a random number in range [1, k]
    1 + (random_value % k)
}

#[no_mangle]
extern "C" fn init() {
    let init_value: PebblesInit = msg::load().expect("Unable to decode PebblesInit");
    initialize_pebbles_init(init_value);
    initialize_pebbles_games();
    msg::reply("", 0).expect("Unable to reply for Init");
}


fn program_easy_turn(game_state: &GameState) -> u32 {
    // For Easy level, Program chooses a random number of pebbles to remove
    get_random_num_1_to_k(game_state.max_pebbles_per_turn)
}

fn program_hard_turn(game_state: &GameState) -> u32 {
    // For Hard level, Program tries to find the optimal number of pebbles to remove
    // Here, we'll implement a simple winning strategy for demonstration purposes
    // In a real scenario, this strategy would be more sophisticated

    // If the remaining pebbles count is less than or equal to max_pebbles_per_turn, remove all remaining pebbles
    if game_state.pebbles_remaining <= game_state.max_pebbles_per_turn {
        game_state.pebbles_remaining
    } else {
        // Otherwise, remove enough pebbles to leave a winning number for the next turn
        (game_state.pebbles_remaining - 1) % (game_state.max_pebbles_per_turn + 1)
    }
}

fn process_program_turn(user_id: ActorId) {
    unsafe {
        if let Some(ref mut games) = PEBBLES_GAMES {
            if let Some(game_state) = games.get_mut(&user_id) {
                let pebbles_to_remove = match game_state.difficulty {
                    DifficultyLevel::Easy => program_easy_turn(game_state),
                    DifficultyLevel::Hard => program_hard_turn(game_state),
                };

                if pebbles_to_remove <= game_state.pebbles_remaining {
                    game_state.pebbles_remaining -= pebbles_to_remove;
                    if game_state.pebbles_remaining == 0 {
                        game_state.winner = Some(Player::Program);
                        remove_game(user_id);
                    }
                } else {
                    // Invalid move (should not happen in this case)
                }
            }
        }
    }
}


#[no_mangle]
extern "C" fn handle() {
    let action: PebblesAction = msg::load().expect("Unable to decode PebblesAction");
    let user_id = msg::source();

    unsafe {
        let pebbles_games = PEBBLES_GAMES.as_mut().expect("Game state not initialized");

        let game_state = pebbles_games.entry(user_id).or_insert_with(|| GameState {
            pebbles_count: get_pebbles_count().expect("get_pebbles_count error"),
            max_pebbles_per_turn: get_max_pebbles_per_turn().expect("get_max_pebbles_per_turn error"),
            pebbles_remaining: get_pebbles_count().expect("get_pebbles_count error"),
            difficulty: get_pebbles_difficulty().expect("get_pebbles_difficulty error"),
            first_player: Player::User,
            winner: None,
        });

        match action {
            PebblesAction::Turn(pebbles) => {
                if pebbles > game_state.max_pebbles_per_turn || pebbles == 0 {
                    // Invalid move
                    msg::reply(PebblesEvent::CounterTurn(0), 0).expect("Unable to reply for Turn");
                } else {
                    // Valid move
                    game_state.pebbles_remaining -= pebbles;
                    if game_state.pebbles_remaining == 0 {
                        game_state.winner = Some(Player::User);
                        remove_game(user_id);
                        msg::reply(PebblesEvent::Won(Player::User), 0).expect("Unable to reply");
                    } else {
                        process_program_turn(user_id);
                    }
                }
            }
            PebblesAction::GiveUp => {
                game_state.winner = Some(Player::Program);
                remove_game(user_id);
                msg::reply(PebblesEvent::Won(Player::Program), 0).expect("Unable to reply");
            }
            PebblesAction::Restart { difficulty, pebbles_count, max_pebbles_per_turn } => {
                game_state.pebbles_count = pebbles_count;
                game_state.max_pebbles_per_turn = max_pebbles_per_turn;
                game_state.pebbles_remaining = pebbles_count;
                game_state.difficulty = difficulty;
                game_state.winner = None;
                game_state.first_player = if get_random_u32() % 2 == 0 {
                    Player::User
                } else {
                    Player::Program
                };

                match game_state.first_player{
                    Player::User => {},
                    Player::Program => {
                        process_program_turn(user_id);
                    }
                }
                msg::reply(game_state, 0).expect("Unable to reply with game state");
            }
        }
    }
}


#[no_mangle]
extern "C" fn state() {
    let user_id = msg::source();

    unsafe {
        let pebbles_games = PEBBLES_GAMES.as_ref().expect("Game state not initialized");

        if let Some(game_state) = pebbles_games.get(&user_id) {
            msg::reply(game_state, 0).expect("Unable to reply with game state");
        }
    }
}


#[no_mangle]
extern "C" fn handle_reply() {}

#[no_mangle]
extern "C" fn handle_signal() {}



