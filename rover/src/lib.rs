#[derive(PartialEq,Debug,Clone)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

#[derive(PartialEq,Debug,Clone)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

#[derive(PartialEq,Debug)]
pub struct Rover {
    pub position: Position,
    pub direction: Direction,
}

fn rotate_right(x: Direction) -> Direction {
    return match x {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
    }
}

fn rotate_left(x: Direction) -> Direction {
    return match x {
        Direction::North => Direction::West,
        Direction::West => Direction::South,
        Direction::South => Direction::East,
        Direction::East => Direction::North,
    }
}

fn move_forward(direction: Direction, position: Position) -> Position {
    return match direction {
        Direction::North => Position { x: position.x, y: (position.y + 1) % 5 },
        Direction::West => Position { x: (position.x + 4) % 5, y: position.y },
        Direction::South => Position { x: position.x, y: (position.y + 4) % 5 },
        Direction::East => Position { x: (position.x + 1) % 5, y: position.y },
    }
}

pub fn rover(commands: &str) -> Rover {
    return roverWithObstacles(commands, &vec![]);
}

pub fn roverWithObstacles(commands: &str, obstacles: &Vec<Position>) -> Rover {
    let valid_position = &|x| {
        return if obstacles.contains(&x) {
            None
        } else {
            Some(x)
        }
    };

    return commands.chars().fold(
        Rover{position: Position{x: 0, y: 0}, direction: Direction::North},
        move |acc, command| 
            match command {
                'M' => Rover{
                        position: valid_position(move_forward(acc.direction.clone(), acc.position.clone())).unwrap_or(acc.position.clone()),
                        direction: acc.direction.clone()
                      },
                'R' => Rover{position: acc.position, direction: rotate_right(acc.direction)},
                'L' => Rover{position: acc.position, direction: rotate_left(acc.direction)},
                _ => panic!("Unknown command: {}", command),
            }
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_commands_should_be_at_00N() {
        assert_eq!(rover(""), Rover{position: Position{x: 0, y: 0}, direction: Direction::North});
    }

    #[test]
    fn M_should_be_at_01N() {
        assert_eq!(rover("M"), Rover{position: Position{x: 0, y: 1}, direction: Direction::North});
    }

    #[test]
    fn MM_should_be_at_02N() {
        assert_eq!(rover("MM"), Rover{position: Position{x: 0, y: 2}, direction: Direction::North});
    }

    #[test]
    fn MMMMM_should_be_at_00N() {
        assert_eq!(rover("MMMMM"), Rover{position: Position{x: 0, y: 0}, direction: Direction::North});
    }

    #[test]
    fn R_should_be_at_00E() {
        assert_eq!(rover("R"), Rover{position: Position{x: 0, y: 0}, direction: Direction::East});
    }

    #[test]
    fn L_should_be_at_00W() {
        assert_eq!(rover("L"), Rover{position: Position{x: 0, y: 0}, direction: Direction::West});
    }

    #[test]
    fn RR_should_be_at_00S() {
        assert_eq!(rover("RR"), Rover{position: Position{x: 0, y: 0}, direction: Direction::South});
    }

    #[test]
    fn LL_should_be_at_00S() {
        assert_eq!(rover("LL"), Rover{position: Position{x: 0, y: 0}, direction: Direction::South});
    }

    #[test]
    fn LLMM_should_be_at_03S() {
        assert_eq!(rover("LLMM"), Rover{position: Position{x: 0, y: 3}, direction: Direction::South});
    }

    #[test]
    fn LMM_should_be_at_30W() {
        assert_eq!(rover("LMM"), Rover{position: Position{x: 3, y: 0}, direction: Direction::West});
    }

    #[test]
    fn RMM_should_be_at_20E() {
        assert_eq!(rover("RMM"), Rover{position: Position{x: 2, y: 0}, direction: Direction::East});
    }

    #[test]
    fn MRMLM_with_obstacle_11_should_be_at_20N() {
        assert_eq!(roverWithObstacles("MRMLM", &vec![Position{x: 1, y: 1}]), Rover{position: Position{x: 0, y: 2}, direction: Direction::North});
    }
}
