use batbox::*;
use logicsider::*;

#[test]
fn main() {
    logger::init_for_tests();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("assets")
        .join("world.ldtk");
    let ldtk: ldtk_json::Ldtk =
        serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path).unwrap()))
            .unwrap();
    for (index, level) in ldtk.levels.iter().enumerate() {
        let level_name = &level.identifier;
        let solutions = level.field_instances.iter().find_map(|field| {
            (field.identifier == "Solutions" && !field.value.is_null()).then_some(&field.value)
        });
        let Some(solutions) = solutions else {
            log::warn!("Level #{index}({level_name}) does not have solutions");
            continue;
        };
        let solutions = solutions
            .as_str()
            .expect("Solutions must be a multi line string");

        for line in solutions.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut game_state = GameState::from_ldtk(&ldtk, index);
            for c in line.chars().chain(".".chars()) {
                match c {
                    '<' => {
                        game_state.process_turn(Input::Left);
                    }
                    '>' => {
                        game_state.process_turn(Input::Right);
                    }
                    '.' => {
                        game_state.process_turn(Input::Skip);
                    }
                    '1'..='9' => {
                        let index = c.to_digit(10).unwrap() as usize;
                        game_state.select_player(index);
                    }
                    _ => panic!("{c:?} character in tests means nothing"),
                }
            }
            assert!(
                game_state.finished(),
                "{line:?} is not a correct solution for level #{index}({level_name})"
            );
        }
    }
}