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
use std::time::{Duration, Instant};

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
    body: VecDeque<Point>,
    /// The current direction the snake is moving.
    direction: Direction,
    /// The next direction to change to (prevents multiple direction changes in one frame)
    next_direction: Option<Direction>,
}

impl Snake {
    /// Creates a new snake with a default starting position and direction.
    fn new(width: u16, height: u16) -> Self {
        let mut body = VecDeque::new();
        // Start the snake in the middle of the board
        body.push_front(Point {
            x: width / 2,
            y: height / 2,
        });
        Self {
            body,
            direction: Direction::Right,
            next_direction: None,
        }
    }

    /// Moves the snake one step forward in its current direction.
    fn move_forward(&mut self, ate_food: bool) {
        // Apply pending direction change if any
        if let Some(dir) = self.next_direction {
            self.direction = dir;
            self.next_direction = None;
        }

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

        // Remove tail if we didn't eat food (snake grows when it eats)
        if !ate_food {
            self.body.pop_back();
        }
    }

    /// Changes the snake's direction.
    fn change_direction(&mut self, new_direction: Direction) {
        // Queue the direction change for the next movement
        if self.direction != new_direction.opposite() {
            self.next_direction = Some(new_direction);
        }
    }

    /// Checks if the snake has collided with itself.
    fn has_collided_with_self(&self) -> bool {
        let head = self.body.front().unwrap();
        self.body.iter().skip(1).any(|segment| segment == head)
    }
}

/// Represents the main game state.
struct Game {
    snake: Snake,
    food: Point,
    score: u32,
    game_over: bool,
    width: u16,
    height: u16,
    last_update: Instant,
    frame_duration: Duration,
}

impl Game {
    /// Creates a new game instance.
    fn new() -> std::io::Result<Self> {
        let (mut width, mut height) = size()?;
        // Ensure minimum playable area
        width = width.max(20);
        height = (height - 1).max(10); // Reserve last row for score

        let mut game = Self {
            snake: Snake::new(width, height),
            food: Point { x: 0, y: 0 },
            score: 0,
            game_over: false,
            width,
            height,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(150),
        };
        game.place_food();
        Ok(game)
    }

    /// Resets the game state
    fn reset(&mut self) {
        self.snake = Snake::new(self.width, self.height);
        self.score = 0;
        self.game_over = false;
        self.place_food();
        self.last_update = Instant::now();
    }

    /// Places the food at a new random location on the board.
    fn place_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let new_food_pos = Point {
                x: rng.gen_range(1..(self.width - 1)),
                y: rng.gen_range(1..(self.height - 1)),
            };
            // Make sure the food is not on the snake's body
            if !self.snake.body.contains(&new_food_pos) {
                self.food = new_food_pos;
                break;
            }
        }
    }

    /// The main game loop.
    fn run(&mut self, stdout: &mut Stdout) -> std::io::Result<()> {
        while !self.game_over {
            self.handle_input()?;

            // Use frame timing for consistent movement speed
            let now = Instant::now();
            if now.duration_since(self.last_update) >= self.frame_duration {
                self.last_update = now;
                self.update_game();
                self.draw(stdout)?;
            }

            // Small sleep to prevent 100% CPU usage
            std::thread::sleep(Duration::from_millis(5));
        }

        self.show_game_over(stdout)?;
        Ok(())
    }

    /// Handles user input for controlling the snake.
    fn handle_input(&mut self) -> std::io::Result<()> {
        while poll(Duration::from_millis(0))? {
            if let Event::Key(key_event) = read()? {
                let new_direction = match key_event.code {
                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => Some(Direction::Up),
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => Some(Direction::Down),
                    KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => Some(Direction::Left),
                    KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => Some(Direction::Right),
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
        if new_head.x == 0
            || new_head.x == self.width - 1
            || new_head.y == 0
            || new_head.y == self.height - 1
            || self.snake.has_collided_with_self()
        {
            self.game_over = true;
        }
    }

    /// Draws the entire game screen.
    fn draw(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        execute!(stdout, Clear(ClearType::All))?;
        self.draw_border(stdout)?;
        self.draw_snake(stdout)?;
        self.draw_food(stdout)?;
        self.draw_score(stdout)?;
        Ok(())
    }

    /// Draws the border of the game board.
    fn draw_border(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        execute!(stdout, SetForegroundColor(Color::DarkGrey))?;
        // Top and bottom borders
        for x in 0..self.width {
            execute!(stdout, MoveTo(x, 0), Print("#"))?;
            execute!(stdout, MoveTo(x, self.height - 1), Print("#"))?;
        }
        // Left and right borders
        for y in 1..self.height - 1 {
            execute!(stdout, MoveTo(0, y), Print("#"))?;
            execute!(stdout, MoveTo(self.width - 1, y), Print("#"))?;
        }
        execute!(stdout, ResetColor)
    }

    /// Draws the snake on the board.
    fn draw_snake(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        for (i, segment) in self.snake.body.iter().enumerate() {
            // Head is different from body
            let symbol = if i == 0 { "O" } else { "o" };
            let color = if i == 0 {
                SetForegroundColor(Color::Green)
            } else {
                SetForegroundColor(Color::DarkGreen)
            };
            execute!(
                stdout,
                color,
                MoveTo(segment.x, segment.y),
                Print(symbol),
                ResetColor
            )?;
        }
        Ok(())
    }

    /// Draws the food on the board.
    fn draw_food(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            MoveTo(self.food.x, self.food.y),
            Print("*"),
            ResetColor
        )
    }

    /// Draws the current score.
    fn draw_score(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        let score_text = format!("Score: {}", self.score);
        execute!(
            stdout,
            MoveTo(1, self.height),
            Print(score_text)
        )
    }

    fn show_game_over(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        let game_over_text = "GAME OVER";
        let score_text = format!("Final Score: {}", self.score);
        let restart_text = "Press 'R' to restart or 'Q' to quit";

        let center_x = self.width / 2;
        let mut y_pos = self.height / 2 - 2;

        execute!(
            stdout,
            Clear(ClearType::All),
            MoveTo(center_x - game_over_text.len() as u16 / 2, y_pos),
            SetForegroundColor(Color::Red),
            Print(game_over_text),
        )?;
        y_pos += 2;

        execute!(
            stdout,
            MoveTo(center_x - score_text.len() as u16 / 2, y_pos),
            SetForegroundColor(Color::Yellow),
            Print(score_text),
        )?;
        y_pos += 2;

        execute!(
            stdout,
            MoveTo(center_x - restart_text.len() as u16 / 2, y_pos),
            SetForegroundColor(Color::Cyan),
            Print(restart_text),
            ResetColor
        )?;

        // Wait for key press to restart or quit
        loop {
            if poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = read()? {
                    match key_event.code {
                        KeyCode::Char('r') | KeyCode::Char('R') => return Ok(()),
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        Hide,
        Clear(ClearType::All)
    )?;

    let mut restart = true;
    while restart {
        let mut game = Game::new()?;
        game.run(&mut stdout)?;

        // Show restart prompt
        execute!(stdout, Clear(ClearType::All))?;
        let restart_text = "Press 'R' to restart or any other key to quit";
        let (width, height) = size()?;
        execute!(
            stdout,
            MoveTo(width / 2 - restart_text.len() as u16 / 2, height / 2),
            Print(restart_text)
        )?;

        // Wait for restart decision
        if poll(Duration::from_secs(1))? {
            if let Event::Key(key_event) = read()? {
                if let KeyCode::Char('r') | KeyCode::Char('R') = key_event.code {
                    continue;
                }
            }
        }
        restart = false;
    }

    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}