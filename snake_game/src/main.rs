// main.rs

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{poll, read, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use rand::Rng;
use std::collections::VecDeque;
use std::io::{stdout, Stdout};
use std::time::Duration;

// The dimensions of our game board
const WIDTH: u16 = 20;
const HEIGHT: u16 = 15;

/// Represents a single point on the 2D game grid.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Point {
    x: u16,
    y: u16,
}

/// Represents the direction the snake can move.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Returns the opposite direction.
    /// This is useful for preventing the snake from reversing into itself.
    fn opposite(&self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

/// Represents the snake in the game.
struct Snake {
    /// The body of the snake, represented as a queue of points.
    /// The head of the snake is at the front of the queue.
    body: VecDeque<Point>,
    /// The current direction the snake is moving.
    direction: Direction,
}

impl Snake {
    /// Creates a new snake with a default starting position and direction.
    fn new() -> Self {
        let mut body = VecDeque::new();
        // Start the snake in the middle of the board
        body.push_front(Point {
            x: WIDTH / 2,
            y: HEIGHT / 2,
        });
        Self {
            body,
            direction: Direction::Right,
        }
    }

    /// Moves the snake one step forward in its current direction.
    /// It also handles whether the snake eats food or just moves.
    fn move_forward(&mut self, ate_food: bool) {
        let head = self.body.front().unwrap();
        let new_head = match self.direction {
            Direction::Up => Point {
                x: head.x,
                y: head.y.saturating_sub(1),
            },
            Direction::Down => Point {
                x: head.x,
                y: head.y + 1,
            },
            Direction::Left => Point {
                x: head.x.saturating_sub(1),
                y: head.y,
            },
            Direction::Right => Point {
                x: head.x + 1,
                y: head.y,
            },
        };

        self.body.push_front(new_head);

        // If the snake did not eat food, remove its tail segment.
        // This makes it look like it's moving.
        // If it did eat food, we don't remove the tail, so it grows.
        if !ate_food {
            self.body.pop_back();
        }
    }

    /// Changes the snake's direction.
    /// The snake cannot move in the opposite direction of its current movement.
    fn change_direction(&mut self, new_direction: Direction) {
        if self.direction != new_direction.opposite() {
            self.direction = new_direction;
        }
    }

    /// Checks if the snake has collided with itself.
    fn has_collided_with_self(&self) -> bool {
        let head = self.body.front().unwrap();
        // We check if any body segment (excluding the head) is at the same position as the head.
        self.body.iter().skip(1).any(|segment| segment == head)
    }
}

/// Represents the main game state.
struct Game {
    stdout: Stdout,
    snake: Snake,
    food: Point,
    score: u32,
    game_over: bool,
}

impl Game {
    /// Creates a new game instance.
    fn new() -> Self {
        let mut game = Self {
            stdout: stdout(),
            snake: Snake::new(),
            food: Point { x: 0, y: 0 }, // Dummy position, will be replaced
            score: 0,
            game_over: false,
        };
        game.place_food();
        game
    }

    /// Places the food at a new random location on the board.
    /// It ensures the food does not appear on the snake's body.
    fn place_food(&mut self) {
        let mut rng = rand::rng();
        loop {
            let new_food_pos = Point {
                x: rng.random_range(1..(WIDTH - 1)),
                y: rng.random_range(1..(HEIGHT - 1)),
            };
            // Make sure the food is not on the snake's body
            if !self.snake.body.contains(&new_food_pos) {
                self.food = new_food_pos;
                break;
            }
        }
    }

    /// The main game loop.
    fn run(&mut self) -> std::io::Result<()> {
        enable_raw_mode()?;
        execute!(
            self.stdout,
            EnterAlternateScreen,
            Hide,
            Clear(ClearType::All)
        )?;

        while !self.game_over {
            self.draw()?;
            self.handle_input()?;
            self.update_game();
            std::thread::sleep(Duration::from_millis(150));
        }

        self.show_game_over()?;

        execute!(self.stdout, Show, LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    /// Handles user input for controlling the snake.
    fn handle_input(&mut self) -> std::io::Result<()> {
        if poll(Duration::from_millis(10))? {
            if let Event::Key(key_event) = read()? {
                let new_direction = match key_event.code {
                    KeyCode::Up | KeyCode::Char('w') => Some(Direction::Up),
                    KeyCode::Down | KeyCode::Char('s') => Some(Direction::Down),
                    KeyCode::Left | KeyCode::Char('a') => Some(Direction::Left),
                    KeyCode::Right | KeyCode::Char('d') => Some(Direction::Right),
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.game_over = true;
                        None
                    }
                    _ => None,
                };
                if let Some(direction) = new_direction {
                    self.snake.change_direction(direction);
                }
            }
        }
        Ok(())
    }

    /// Updates the game state on each tick.
    fn update_game(&mut self) {
        let head = self.snake.body.front().unwrap();

        let ate_food = *head == self.food;

        self.snake.move_forward(ate_food);

        if ate_food {
            self.score += 1;
            self.place_food();
        }

        let new_head = self.snake.body.front().unwrap();
        if new_head.x == 0 || new_head.x == WIDTH -1 || new_head.y == 0 || new_head.y == HEIGHT -1 || self.snake.has_collided_with_self() {
            self.game_over = true;
        }
    }

    /// Draws the entire game screen.
    fn draw(&mut self) -> std::io::Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;
        self.draw_border()?;
        self.draw_snake()?;
        self.draw_food()?;
        self.draw_score()?;
        Ok(())
    }

    /// Draws the border of the game board.
    fn draw_border(&mut self) -> std::io::Result<()> {
        execute!(self.stdout, SetForegroundColor(Color::Grey))?;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if x == 0 || x == WIDTH - 1 || y == 0 || y == HEIGHT - 1 {
                    execute!(self.stdout, MoveTo(x, y), Print("█"))?;
                }
            }
        }
        execute!(self.stdout, ResetColor)
    }

    /// Draws the snake on the board.
    fn draw_snake(&mut self) -> std::io::Result<()> {
        execute!(self.stdout, SetForegroundColor(Color::Green))?;
        for (i, segment) in self.snake.body.iter().enumerate() {
            let symbol = if i == 0 { "■" } else { "■" }; // Head and body
            execute!(self.stdout, MoveTo(segment.x, segment.y), Print(symbol))?;
        }
        execute!(self.stdout, ResetColor)
    }

    /// Draws the food on the board.
    fn draw_food(&mut self) -> std::io::Result<()> {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Red),
            MoveTo(self.food.x, self.food.y),
            Print("🍎"),
            ResetColor
        )
    }

    /// Draws the current score.
    fn draw_score(&mut self) -> std::io::Result<()> {
        let score_text = format!("Score: {}", self.score);
        execute!(self.stdout, MoveTo(1, HEIGHT), Print(score_text))
    }

    fn show_game_over(&mut self) -> std::io::Result<()> {
        execute!(self.stdout, Clear(ClearType::All))?;
        let (width, height) = size()?;
        let game_over_text = "Game Over!";
        let score_text = format!("Final Score: {}", self.score);
        let quit_text = "Press 'q' to quit.";

        execute!(
            self.stdout,
            MoveTo(width / 2 - game_over_text.len() as u16 / 2, height / 2 - 1),
            Print(game_over_text),
            MoveTo(width / 2 - score_text.len() as u16 / 2, height / 2),
            Print(score_text),
            MoveTo(width / 2 - quit_text.len() as u16 / 2, height / 2 + 1),
            Print(quit_text)
        )?;

        // Wait for 'q' to be pressed to exit
        loop {
            if poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = read()? {
                    if key_event.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}


fn main() -> std::io::Result<()> {
    let mut game = Game::new();
    game.run()
}