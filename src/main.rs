use crossterm::style::Print;
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{cursor, execute, queue};
use std::collections::HashMap;
use std::io::{stdin, stdout, Stdout, Write};
use std::time::{Duration, Instant};

///Formats and prints each argument with a new line to a given writer
macro_rules! printnl {
    ($writer:expr, $($text:expr),+ $(,)?) => {{
        $(
            execute!(
                $writer,
                Print(format!($text)),
                Clear(ClearType::UntilNewLine),
                cursor::MoveToNextLine(1)
            ).unwrap();
        )+
    }};
}

fn main() {
    let width = (size().unwrap().0 / 2) as usize;
    let height = (size().unwrap().1 - 7) as usize;
    let mut board = random_board(width, height);
    let mut generation = 0usize;
    let mut board_history = HashMap::new();
    let mut stdout = stdout();
    let frame_time = get_fps();

    queue!(stdout, Clear(ClearType::All), cursor::Hide).unwrap(); //prep console

    let start_time = Instant::now();

    loop {
        queue!(stdout, cursor::MoveTo(0, 0)).unwrap();
        print_board(&board, &mut stdout);
        print_stats(&board, generation, &mut stdout);
        match detect_loop(&board, generation, &mut board_history) {
            Some(loop_start) => {
                printnl!(stdout, "Game started looping from generation {loop_start}");
                break;
            }
            None => {
                iterate_board(&mut board);
                generation += 1;
            }
        }
        std::thread::sleep(frame_time);
    }

    //print average fps if unlimited
    if frame_time.as_millis() == 0 {
        execute!(
            stdout,
            Print(format!(
                "FPS: ~{:.2}",
                generation as f64 / start_time.elapsed().as_secs_f64()
            )),
            Clear(ClearType::UntilNewLine),
        ).unwrap();
    }
    //cleanup
    execute!(
        stdout,
        cursor::MoveToNextLine(1),
        cursor::Show
    ).unwrap();
}

///Generates a random board with a given width and height
fn random_board(width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut board = vec![vec![false; width]; height];
    for x in 0..width {
        for y in 0..height {
            if fastrand::bool() {
                board[y][x] = true;
            }
        }
    }
    board
}

fn get_fps() -> Duration {
    const DEFAULT_FPS: u64 = 20;
    print!("Input the desired FPS (0 for unlimited, default {DEFAULT_FPS}): ");
    stdout().flush().unwrap();
    let mut input_text = String::new();
    stdin().read_line(&mut input_text).unwrap();
    let target_fps = input_text.trim().parse::<u64>().unwrap_or(DEFAULT_FPS);
    let frame_time: Duration;
    if target_fps > 0 {
        frame_time = Duration::from_millis(1000 / target_fps);
    } else {
        frame_time = Duration::from_millis(0);
    }
    frame_time
}

const NEIGHBOR_OFFSETS: [(isize, isize); 8] = [
    (-1, -1), (-1, 0), (-1, 1),
    ( 0, -1),          ( 0, 1),
    ( 1, -1), ( 1, 0), ( 1, 1),
];

///Takes a board and iterates it according to the rules of the game
fn iterate_board(board: &mut Vec<Vec<bool>>) {
    let height = board.len();
    let width = board[0].len();
    let mut new_board = vec![vec![false; width]; height];

    for y in 0..height {
        for x in 0..width {
            let mut neighbors = 0;
            //check neighboring cells
            for (y_offset, x_offset) in NEIGHBOR_OFFSETS.iter() {
                let y_pos = y as isize + y_offset;
                let x_pos = x as isize + x_offset;
                if y_pos >= 0
                    && x_pos >= 0
                    && y_pos < height as isize
                    && x_pos < width as isize
                    && board[y_pos as usize][x_pos as usize]
                {
                    neighbors += 1;
                }
            }
            //check if cell lives
            if board[y][x] && neighbors > 1 && neighbors < 4 {
                new_board[y][x] = true;
            } else if !board[y][x] && neighbors == 3 {
                new_board[y][x] = true;
            }
        }
    }
    board.clone_from(&new_board);
}

///Prints a board to stdout
fn print_board(board: &Vec<Vec<bool>>, stdout: &mut Stdout) {
    let mut buffer = String::with_capacity((board[0].len() * 2 + 1) * board[0].len());
    for y in 0..board.len() {
        for x in 0..board[0].len() {
            let pixel = if board[y][x] { "██" } else { "  " };
            buffer.push_str(pixel);
        }
        buffer.push('\n');
    }
    execute!(stdout, Print(buffer)).unwrap();
}

///Prints board statistics
fn print_stats(board: &Vec<Vec<bool>>, generation: usize, stdout: &mut Stdout) {
    let mut population = 0u128;
    for y in 0..board.len() {
        for x in 0..board[0].len() {
            if board[y][x] {
                population += 1;
            }
        }
    }
    printnl!(
        stdout,
        "Generation: {generation}",
        "Population: {population}"
    );
}

///Detects looping via hashmap of previous board states and returns the generation the loop started
fn detect_loop(
    board: &Vec<Vec<bool>>,
    generation: usize,
    history: &mut HashMap<Vec<bool>, usize>,
) -> Option<usize> {
    history
        .insert(board.iter().flatten().cloned().collect(), generation)
        .and_then(Some)
}
